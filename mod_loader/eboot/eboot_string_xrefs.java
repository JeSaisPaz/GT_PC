//@category: GT PSP
//@author: Mod Loader
//@description: Extract string cross-references from decrypted EBOOT for VFS analysis

import java.io.File;
import java.io.FileWriter;
import java.util.HashSet;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;

public class eboot_string_xrefs extends GhidraScript {

    @Override
    protected void run() throws Exception {
        println("=== EBOOT String XREF Analyzer ===");
        println("Program: " + currentProgram.getName());

        String[] targets = {
            "bootstrap",
            "packed_main_loop",
            "bootstrap_phase2",
            "shutdown",
            "Application",
            "init_sound",
            ".adc",
            "GT.VOL",
            "gt5m",
            "projects/",
            "scripts/",
            "products/",
            "MainLoop",
            "SpecDB",
            "pdiext",
            "load",
            "PROJECT_ROOT_DIR",
        };

        StringBuilder sb = new StringBuilder();
        sb.append("{\n");
        sb.append("  \"program\": \"").append(currentProgram.getName()).append("\",\n");
        sb.append("  \"language\": \"").append(currentProgram.getLanguageID()).append("\",\n");
        sb.append("  \"strings\": [\n");

        int first = 0;
        Listing listing = currentProgram.getListing();
        SymbolTable symTable = currentProgram.getSymbolTable();

        // Scan all defined data for strings
        DataIterator dataIter = listing.getDefinedData(true);
        int stringCount = 0;

        while (dataIter.hasNext() && stringCount < 200) {
            Data data = dataIter.next();
            if (!data.isString()) continue;

            String val = data.getDefaultValueRepresentation();
            if (val == null || val.length() < 3) continue;

            String lower = val.toLowerCase();
            boolean matched = false;
            for (String t : targets) {
                if (lower.contains(t.toLowerCase())) {
                    matched = true;
                    break;
                }
            }
            if (!matched) continue;

            Address addr = data.getAddress();
            if (first++ > 0) sb.append(",\n");
            sb.append("    {\n");
            sb.append("      \"address\": \"").append(addr.toString()).append("\",\n");
            sb.append("      \"offset\": \"0x").append(addr.getOffset()).append("\",\n");
            sb.append("      \"value\": \"").append(escapeJson(val)).append("\",\n");
            sb.append("      \"length\": ").append(val.length()).append(",\n");

            // Get cross-references to this string
            Reference[] refs = symTable.getReferences(addr);
            sb.append("      \"xrefs\": [\n");
            int rf = 0;
            Set<String> seenFuncs = new HashSet<>();
            for (Reference ref : refs) {
                Address fromAddr = ref.getFromAddress();
                Function func = listing.getFunctionContaining(fromAddr);
                String funcName = func != null ? func.getName() : "unknown";
                String funcAddr = func != null ? func.getEntryPoint().toString() : fromAddr.toString();

                // Deduplicate by function
                String key = funcName + "@" + funcAddr;
                if (seenFuncs.contains(key)) continue;
                seenFuncs.add(key);

                if (rf++ > 0) sb.append(",\n");
                sb.append("        {\n");
                sb.append("          \"caller_address\": \"").append(fromAddr.toString()).append("\",\n");
                if (func != null) {
                    sb.append("          \"function_name\": \"").append(escapeJson(func.getName())).append("\",\n");
                    sb.append("          \"function_entry\": \"").append(func.getEntryPoint().toString()).append("\",\n");
                    sb.append("          \"function_size\": ").append(func.getBody().getNumAddresses()).append("\n");
                } else {
                    sb.append("          \"function_name\": null,\n");
                    sb.append("          \"function_entry\": null,\n");
                    sb.append("          \"function_size\": 0\n");
                }
                sb.append("        }");
            }
            sb.append("\n      ],\n");
            sb.append("      \"xref_count\": ").append(seenFuncs.size()).append("\n");
            sb.append("    }");
            stringCount++;
        }

        sb.append("\n  ]\n}\n");

        // Write to file
        File outputFile = askFile("Save XREF results", "Save");
        if (outputFile != null) {
            try (FileWriter fw = new FileWriter(outputFile)) {
                fw.write(sb.toString());
            }
            println("Results written to: " + outputFile.getAbsolutePath());
        }

        println("\n=== Summary ===");
        println("Strings found: " + stringCount);
        println("Done.");
    }

    private String escapeJson(String s) {
        if (s == null) return "";
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }
}
