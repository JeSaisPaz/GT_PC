// PPSSPP WebSocket VFS Analyzer for GT PSP EBOOT
// Monitors sceIoOpen calls, identifies the Adhoc VFS load function
// by tracing file opens of known game scripts.

const WebSocket = require('ws');
const fetch = require('cross-fetch');
const fs = require('fs');
const path = require('path');

const SUB_PROTOCOL = 'debugger.ppsspp.org';
const PPSSPP_MATCH_API = '//report.ppsspp.org/match/list';
const WS_PATH = '/debugger';

const VFS_ADDRESSES_PATH = path.join(__dirname, 'vfs_addresses.json');

// Scripts we're looking for - these confirm we found the VFS load function
const TARGET_SCRIPTS = [
    'bootstrap.adc',
    'packed_main_loop.adc',
    'bootstrap_phase2.adc',
    'shutdown.adc',
    'Application.adc',
    'init_sound.adc',
];

let sceIoOpenAddr = null;
let targetCandidates = [];
let allFileOpens = [];

class PPSSPPClient {
    constructor() {
        this.socket = null;
        this.pending = {};
        this.listeners = {};
        this.ticketCounter = 0;
    }

    autoConnect() {
        return fetch(PPSSPP_MATCH_API)
            .then(r => r.json())
            .then(listing => this._tryConnect(listing, 0));
    }

    _tryConnect(listing, idx) {
        if (idx >= listing.length) throw new Error('Could not connect to PPSSPP');
        const uri = 'ws://' + listing[idx].ip + ':' + listing[idx].p + WS_PATH;
        return this.connect(uri).catch(() => this._tryConnect(listing, idx + 1));
    }

    connect(uri) {
        return new Promise((resolve, reject) => {
            const ws = new WebSocket(uri, SUB_PROTOCOL);
            ws.onopen = () => {
                this.socket = ws;
                this._setup();
                resolve();
            };
            ws.onerror = () => reject(new Error('Connection failed'));
            ws.onclose = () => reject(new Error('Connection closed'));
        });
    }

    _setup() {
        this.socket.onmessage = (e) => {
            const data = JSON.parse(e.data);
            if (data.event === 'error') {
                console.error('[PPSSPP]', data.message);
                return;
            }
            if (data.ticket && this.pending[data.ticket]) {
                this.pending[data.ticket](data);
                delete this.pending[data.ticket];
            }
            if (this.listeners[data.event]) {
                this.listeners[data.event].forEach(fn => fn(data));
            }
            if (this.listeners['*']) {
                this.listeners['*'].forEach(fn => fn(data));
            }
        };
    }

    send(event) {
        const noResponse = ['cpu.stepping', 'cpu.resume', 'cpu.breakpoint.add'];
        if (noResponse.includes(event.event)) {
            this.socket.send(JSON.stringify(event));
            return Promise.resolve(null);
        }
        return new Promise((resolve, reject) => {
            const ticket = String(++this.ticketCounter);
            this.pending[ticket] = resolve;
            this.socket.send(JSON.stringify({ ...event, ticket }));
            setTimeout(() => {
                if (this.pending[ticket]) {
                    delete this.pending[ticket];
                    reject(new Error('Timeout waiting for response'));
                }
            }, 30000);
        });
    }

    on(event, fn) {
        if (!this.listeners[event]) this.listeners[event] = [];
        this.listeners[event].push(fn);
    }

    close() {
        if (this.socket) this.socket.close();
    }
}

async function readStringAt(ppsspp, addr) {
    const result = await ppsspp.send({ event: 'memory.readString', address: addr });
    return result.value;
}

async function analyze() {
    const ppsspp = new PPSSPPClient();

    try {
        console.log('Connecting to PPSSPP...');
        await ppsspp.autoConnect();
        console.log('Connected.');

        // Version handshake
        const ver = await ppsspp.send({ event: 'version', name: 'vfs-analyzer', version: '1.0.0' });
        console.log(`PPSSPP ${ver.version} (${ver.name})`);

        // Wait for game to be loaded
        console.log('Waiting for game to load...');
        let gameStatus;
        while (true) {
            gameStatus = await ppsspp.send({ event: 'game.status' });
            if (gameStatus.game) break;
            await new Promise(r => setTimeout(r, 1000));
        }
        console.log(`Game loaded: ${gameStatus.game.id} - ${gameStatus.game.title}`);

        // List HLE functions to find sceIoOpen stub
        console.log('Searching for sceIoOpen stub...');
        const funcList = await ppsspp.send({ event: 'hle.func.list' });
        const sceIoOpen = funcList.functions.find(f => f.name === 'zz_sceIoOpen');
        if (!sceIoOpen) {
            console.error('ERROR: zz_sceIoOpen not found. Game might not be running.');
            console.log('Available functions with "sceIo" in name:');
            funcList.functions.filter(f => f.name.includes('sceIo')).forEach(f =>
                console.log(`  0x${f.address.toString(16).padStart(8, '0')} ${f.name} (${f.size}B)`)
            );
            return;
        }

        sceIoOpenAddr = sceIoOpen.address;
        const addrStr = '0x' + sceIoOpenAddr.toString(16).padStart(8, '0');
        console.log(`Found zz_sceIoOpen at ${addrStr}`);

        // Also find sceKernelAllocatePartitionMemory for heap analysis
        const sceAlloc = funcList.functions.find(f => f.name === 'zz_sceKernelAllocatePartitionMemory');
        if (sceAlloc) {
            console.log(`Found zz_sceKernelAllocatePartitionMemory at 0x${sceAlloc.address.toString(16).padStart(8, '0')}`);
        }

        // Set breakpoint on sceIoOpen
        // Must register the breakpoint listener BEFORE setting the bp
        // because bp hit fires cpu.stepping broadcast immediately
        console.log('Setting breakpoint...');
        await ppsspp.send({ event: 'cpu.breakpoint.add', address: sceIoOpenAddr });
        console.log('Breakpoint set.');

        // Register handler for breakpoint hits
        ppsspp.on('cpu.stepping', async (info) => {
            if (info.pc !== sceIoOpenAddr) return;

            try {
                const a0 = await ppsspp.send({ event: 'cpu.getReg', name: 'a0' });
                const ra = await ppsspp.send({ event: 'cpu.getReg', name: 'ra' });
                const filename = await readStringAt(ppsspp, a0.uintValue);

                const callerAddr = ra.uintValue;
                const callerStr = '0x' + callerAddr.toString(16).padStart(8, '0');
                const entry = {
                    filename,
                    caller_address: callerAddr,
                    caller_hex: callerStr,
                    timestamp: Date.now(),
                };
                allFileOpens.push(entry);

                // Check if this is a target script
                const isTarget = TARGET_SCRIPTS.some(s => filename.includes(s));
                if (isTarget) {
                    console.log(`*** TARGET: ${filename}`);
                    console.log(`    Caller: ${callerStr}`);

                    // Try to get backtrace
                    try {
                        const bt = await ppsspp.send({ event: 'hle.backtrace' });
                        console.log(`    Stack (${bt.frames.length} frames):`);
                        bt.frames.forEach((f, i) => {
                            console.log(`      #${i} 0x${f.pc.toString(16).padStart(8, '0')}  ${f.code || ''}`);
                        });
                    } catch (e) { /* backtrace may fail */ }

                    targetCandidates.push({
                        filename,
                        caller_address: callerAddr,
                        caller_hex: callerStr,
                    });
                } else if (filename.endsWith('.adc')) {
                    console.log(`SCRIPT: ${filename}  <- ${callerStr}`);
                } else if (filename.includes('GT.VOL') || filename.includes('.vol')) {
                    console.log(`VOLUME: ${filename}`);
                }

                // Resume execution
                await ppsspp.send({ event: 'cpu.resume' });
            } catch (e) {
                console.error('Error at breakpoint:', e.message);
                try { await ppsspp.send({ event: 'cpu.resume' }); } catch (_) {}
            }
        });

        // Keep running until user interrupts
        await new Promise(() => {});

    } catch (e) {
        console.error('Fatal error:', e);
    } finally {
        ppsspp.close();
        saveResults();
    }
}

function saveResults() {
    if (targetCandidates.length === 0 && allFileOpens.length === 0) return;

    // Determine the most likely VFS load function:
    // The caller of bootstrap.adc is the most likely candidate
    const results = {
        last_updated: new Date().toISOString(),
        total_file_opens_captured: allFileOpens.length,
        vfs_candidates: targetCandidates,
        vfs_load_function: null,
        sce_io_open_stub: {
            address: '0x' + (sceIoOpenAddr ? sceIoOpenAddr.toString(16).padStart(8, '0') : 'unknown'),
            found: sceIoOpenAddr !== null,
        },
        all_script_loads: allFileOpens
            .filter(e => e.filename.endsWith('.adc'))
            .map(e => ({
                script: e.filename,
                caller: e.caller_hex,
            })),
    };

    // The VFS load function for scripts is likely:
    // 1. The caller when bootstrap.adc is opened
    // 2. The same caller should appear for multiple scripts
    const bootstrapCaller = targetCandidates.find(c => c.filename.includes('bootstrap'));
    if (bootstrapCaller) {
        results.vfs_load_function = {
            address: bootstrapCaller.caller_hex,
            found: true,
            discovered_via: `PPSSPP WebSocket: breakpoint on sceIoOpen, traced caller when "${bootstrapCaller.filename}" opened`,
            confidence: 'high',
        };
    }

    console.log('\n\n=== ANALYSIS RESULTS ===');
    console.log(JSON.stringify(results, null, 2));

    fs.writeFileSync(VFS_ADDRESSES_PATH, JSON.stringify(results, null, 2));
    console.log(`\nResults saved to ${VFS_ADDRESSES_PATH}`);
}

process.on('SIGINT', () => {
    console.log('\nShutting down...');
    saveResults();
    process.exit(0);
});

analyze();
