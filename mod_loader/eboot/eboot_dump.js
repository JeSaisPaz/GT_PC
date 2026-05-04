// PPSSPP EBOOT Decrypted Memory Dump
// Connects to PPSSPP via WebSocket API, waits for game boot,
// then dumps the decrypted PSP memory (0x08800000-0x0A000000) to file.
//
// Usage:
//   1. Launch PPSSPP with GT PSP ISO loaded and running
//   2. node mod_loader/eboot/eboot_dump.js
//   3. Output: decrypted/ directory with .bin, .json, .asm files

const WebSocket = require('ws');
const fetch = require('cross-fetch');
const fs = require('fs');
const path = require('path');

const SUB_PROTOCOL = 'debugger.ppsspp.org';
const PPSSPP_MATCH_API = '//report.ppsspp.org/match/list';
const WS_PATH = '/debugger';

const PSP_RAM_START = 0x08800000;
const PSP_RAM_END   = 0x0A000000;
const PSP_RAM_SIZE  = PSP_RAM_END - PSP_RAM_START;

const OUTPUT_DIR = path.join(__dirname, '..', '..', 'test_output', 'decrypted');

class PPSSPPClient {
    constructor() {
        this.socket = null;
        this.pending = {};
        this.listeners = {};
        this.ticketCounter = 0;
        this.buffer = {};
    }

    autoConnect() {
        return fetch(PPSSPP_MATCH_API)
            .then(r => r.json())
            .then(listing => this._tryConnect(listing, 0));
    }

    _tryConnect(listing, idx) {
        if (idx >= listing.length) throw new Error('No PPSSPP instance found');
        const uri = 'ws://' + listing[idx].ip + ':' + listing[idx].p + WS_PATH;
        return this.connect(uri).catch(() => this._tryConnect(listing, idx + 1));
    }

    connect(uri) {
        return new Promise((resolve, reject) => {
            const ws = new WebSocket(uri, SUB_PROTOCOL);
            ws.onopen = () => { this.socket = ws; this._setup(); resolve(); };
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
                this.listeners[data.event].forEach(f => fn(data));
            }
        };
    }

    send(event, timeout = 30000) {
        return new Promise((resolve, reject) => {
            const ticket = String(++this.ticketCounter);
            this.pending[ticket] = resolve;
            this.socket.send(JSON.stringify({ ...event, ticket }));
            setTimeout(() => {
                if (this.pending[ticket]) {
                    delete this.pending[ticket];
                    reject(new Error('Timeout'));
                }
            }, timeout);
        });
    }

    close() { if (this.socket) this.socket.close(); }
}

async function readMemory(ppsspp, address, size) {
    const result = await ppsspp.send({ event: 'memory.read', address, size });
    // result.value or result.base64 is base64 encoded
    const b64 = result.value || result.base64;
    if (b64) {
        return Buffer.from(b64, 'base64');
    }
    return null;
}

async function dumpDecryptedEboot() {
    console.log('=== PPSSPP EBOOT Decrypted Memory Dump ===\n');

    fs.mkdirSync(OUTPUT_DIR, { recursive: true });

    const ppsspp = new PPSSPPClient();

    try {
        console.log('1. Connecting to PPSSPP...');
        await ppsspp.autoConnect();
        console.log('   Connected.');

        const ver = await ppsspp.send({ event: 'version', name: 'eboot-dump', version: '1.0.0' });
        console.log(`   PPSSPP ${ver.version}`);

        console.log('2. Waiting for game to load...');
        let gameStatus;
        while (true) {
            gameStatus = await ppsspp.send({ event: 'game.status' });
            if (gameStatus.game && !gameStatus.paused) break;
            await new Promise(r => setTimeout(r, 1000));
        }
        console.log(`   Game: ${gameStatus.game.id} - ${gameStatus.game.title}`);

        await new Promise(r => setTimeout(r, 2000));
        console.log('\n3. Locating EBOOT in memory (searching for "PDIAPP")...');

        // First, locate the EBOOT module by searching for its name
        let ebootBase = PSP_RAM_START;
        let ebootFound = false;

        // Try known addresses for PRX module info (usually near 0x08800000 + offset)
        // The PSP EBOOT loader places module info at the entry point area
        const probeAddrs = [
            0x08800000, 0x08804000, 0x08810000, 0x08820000,
            0x08840000, 0x08880000, 0x08900000, 0x08A00000,
            0x08B00000, 0x08C00000, 0x08D00000, 0x08E00000,
            0x08F00000, 0x09000000, 0x09400000, 0x09800000,
        ];

        for (const probe of probeAddrs) {
            const data = await readMemory(ppsspp, probe, 256);
            if (data) {
                const idx = data.indexOf('PDIAPP');
                if (idx >= 0) {
                    ebootBase = probe;
                    ebootFound = true;
                    console.log(`   Found "PDIAPP" at 0x${probe.toString(16)} (offset ${idx})`);
                    break;
                }
                // Also check for ELF magic or PRX magic
                const magic = data.slice(0, 4).toString('hex');
                if (magic !== '00000000') {
                    console.log(`   Non-zero data at 0x${probe.toString(16)}: magic=0x${magic}`);
                }
            }
        }

        if (!ebootFound) {
            console.log('   "PDIAPP" not found in probe addresses. Scanning wider...');
            // Scan first 16MB for the string
            for (let addr = PSP_RAM_START; addr < PSP_RAM_START + 0x1000000; addr += 0x10000) {
                const data = await readMemory(ppsspp, addr, 256);
                if (data && data.indexOf('PDIAPP') >= 0) {
                    ebootBase = addr;
                    ebootFound = true;
                    console.log(`   Found "PDIAPP" at 0x${addr.toString(16)}`);
                    break;
                }
            }
        }

        if (!ebootFound) {
            console.log('   WARNING: Could not locate "PDIAPP" string. Dumping full RAM anyway.');
        }

        // Determine dump range: at least 8MB around the EBOOT, or full RAM
        const dumpStart = ebootBase;
        const dumpEnd = Math.min(ebootBase + 8 * 1024 * 1024, PSP_RAM_END);
        const dumpSize = dumpEnd - dumpStart;

        console.log(`\n4. Dumping decrypted memory (${(dumpSize / 1024 / 1024).toFixed(1)} MB)...`);
        console.log(`   Range: 0x${dumpStart.toString(16)} - 0x${dumpEnd.toString(16)}`);

        // Read in 256KB chunks
        const CHUNK_SIZE = 256 * 1024;
        const chunks = [];
        let totalRead = 0;

        for (let addr = dumpStart; addr < dumpEnd; addr += CHUNK_SIZE) {
            const size = Math.min(CHUNK_SIZE, dumpEnd - addr);
            const chunk = await readMemory(ppsspp, addr, size);
            if (chunk && chunk.length > 0) {
                chunks.push({ address: addr, data: chunk, size: chunk.length });
                totalRead += chunk.length;
            }
            process.stdout.write(`\r   ${((addr - dumpStart) / 1024 / 1024).toFixed(1)} / ${(dumpSize / 1024 / 1024).toFixed(1)} MB...`);
        }
        process.stdout.write(`\r   ${(totalRead / 1024 / 1024).toFixed(1)} MB read. Done.\n`);

        // Merge chunks into single buffer
        const fullDump = Buffer.concat(chunks.map(c => c.data));
        const dumpPath = path.join(OUTPUT_DIR, 'decrypted_eboot.bin');
        fs.writeFileSync(dumpPath, fullDump);
        console.log(`\n5. Raw dump saved: ${dumpPath}`);
        console.log(`   ${fullDump.length} bytes`);

        // Validate — first 4 bytes should be ELF magic or PSP PRX magic
        const magic = fullDump.slice(0, 4).toString('hex');
        const magicReadable = fullDump.slice(0, 4).toString('ascii').replace(/[^\x20-\x7E]/g, '.');
        console.log(`   Magic: 0x${magic} ("${magicReadable}")`);

        // Check for known strings to verify decryption
        const searchStrings = ['bootstrap', 'packed_main_loop', 'GT.VOL', 'PDIAPP', '.adc', 'gt5m', 'MainLoop'];
        console.log('\n6. Scanning dump for key strings:');
        searchStrings.forEach(s => {
            const idx = fullDump.indexOf(s);
            if (idx >= 0) {
                const addr = PSP_RAM_START + idx;
                console.log(`   [FOUND] "${s}" at offset 0x${idx.toString(16)} (RAM 0x${addr.toString(16)})`);
            } else {
                console.log(`   [MISS]  "${s}" not found`);
            }
        });

        // Dump HLE function symbols
        console.log('\n7. Retrieving HLE function symbols...');
        const funcList = await ppsspp.send({ event: 'hle.func.list' });
        const funcsPath = path.join(OUTPUT_DIR, 'decrypted_symbols.json');
        fs.writeFileSync(funcsPath, JSON.stringify(funcList.functions, null, 2));
        console.log(`   ${funcList.functions.length} functions saved to ${funcsPath}`);

        // Generate MIPS disassembly of key areas
        console.log('\n8. Disassembling entry points...');

        // Find key functions from symbol list
        const keyFuncs = funcList.functions.filter(f =>
            f.name.includes('sceIo') ||
            f.name.includes('bootstrap') ||
            f.name.includes('load') ||
            f.name.includes('MainLoop') ||
            f.name.includes('Adhoc') ||
            f.name === 'main'
        );

        const asmLines = [];
        asmLines.push('; PPSSPP Decrypted EBOOT Disassembly Export');
        asmLines.push(`; Game: ${gameStatus.game.id}`);
        asmLines.push(`; Dump range: 0x${PSP_RAM_START.toString(16)}-0x${PSP_RAM_END.toString(16)}\n`);

        // Disassemble key functions
        for (const func of keyFuncs.slice(0, 20)) {
            try {
                const disasm = await ppsspp.send({
                    event: 'memory.disasm',
                    address: func.address,
                    count: Math.min(Math.floor(func.size / 4), 20),
                }, 15000);
                asmLines.push(`; === ${func.name} @ 0x${func.address.toString(16).padStart(8, '0')} (${func.size}B) ===`);
                if (disasm.lines) {
                    disasm.lines.forEach(l => {
                        const name = l.name || '???';
                        const params = l.params || '';
                        asmLines.push(`  0x${l.address.toString(16).padStart(8, '0')}  ${name} ${params}`);
                    });
                }
                asmLines.push('');
            } catch (e) {
                asmLines.push(`; SKIP ${func.name}: ${e.message}`);
            }
        }

        // Disassemble entry point area (PRX header)
        asmLines.push('; === EBOOT Entry Area (0x08800000 - 0x08801000) ===');
        try {
            const entryDisasm = await ppsspp.send({
                event: 'memory.disasm',
                address: PSP_RAM_START,
                count: 128,
            }, 15000);
            if (entryDisasm.lines) {
                entryDisasm.lines.forEach(l => {
                    const name = l.name || '???';
                    const params = l.params || '';
                    asmLines.push(`  0x${l.address.toString(16).padStart(8, '0')}  ${name} ${params}`);
                });
            }
        } catch (e) {
            asmLines.push(`; SKIP entry: ${e.message}`);
        }

        const asmPath = path.join(OUTPUT_DIR, 'decrypted_disasm.asm');
        fs.writeFileSync(asmPath, asmLines.join('\n'));
        console.log(`   Disassembly saved to ${asmPath}`);

        // Resume CPU
        await ppsspp.send({ event: 'cpu.resume' });

        console.log('\n=== DUMP COMPLETE ===');
        console.log(`Output directory: ${OUTPUT_DIR}`);
        console.log('Files:');
        console.log(`  decrypted_eboot.bin     - ${(fullDump.length / 1024 / 1024).toFixed(1)} MB raw RAM dump`);
        console.log(`  decrypted_symbols.json  - ${funcList.functions.length} function symbols`);
        console.log(`  decrypted_disasm.asm    - Entry point + key functions disassembly`);

        console.log('\nNext steps:');
        console.log('  1. Import decrypted_eboot.bin into Ghidra as Raw Binary (MIPS:LE:32:default)');
        console.log('  2. Apply symbol map from decrypted_symbols.json');
        console.log('  3. Search strings for "bootstrap", ".adc", "GT.VOL" to find VFS');
        console.log('  4. Run vfs_analyzer.js for automated VFS call tracing');

    } catch (e) {
        console.error('\nERROR:', e.message);
    } finally {
        ppsspp.close();
    }
}

dumpDecryptedEboot();
