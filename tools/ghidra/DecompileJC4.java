// Export decompiled C for every function in Just Cause 4 (JustCause4.exe, Avalanche Apex engine).
// Adapted from DecompileSaboteur.java (Pandemic/WildStar) — but JC4 is x64, so addresses are 64-bit:
// every VA is printed as 0x%x (not the 32-bit %08x the Saboteur/Mercs2 scripts used), otherwise the
// image-based addresses (base 0x140000000) would be truncated.
// Each function is emitted once (dedup by entry), sorted by address, with a header line:
//   ==== <name> @0xADDR  size=N  callers=[...] ====
// Writes output/_ghidra_jc4/jc4_all_functions_decomp.txt
//
// @category JC4
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;
import ghidra.util.task.ConsoleTaskMonitor;

import java.io.File;
import java.io.PrintWriter;
import java.util.LinkedHashSet;
import java.util.Set;
import java.util.TreeMap;

public class DecompileJC4 extends GhidraScript {
    private static final String OUT_DIR =
        "C:\\Users\\Shadow\\Desktop\\notes-on-reversing-jc4\\output\\_ghidra_jc4\\";

    private PrintWriter fp;
    private DecompInterface decomp;
    private ConsoleTaskMonitor mon;
    private final Set<Long> done = new LinkedHashSet<>();

    private String callersOf(Function f) {
        StringBuilder sb = new StringBuilder();
        int n = 0;
        for (Reference r : getReferencesTo(f.getEntryPoint())) {
            if (!r.getReferenceType().isCall()) continue;
            Function c = getFunctionContaining(r.getFromAddress());
            sb.append(String.format("0x%x%s ", r.getFromAddress().getOffset(),
                c != null ? "(" + c.getName() + ")" : ""));
            if (++n >= 12) { sb.append("..."); break; }
        }
        return sb.toString().trim();
    }

    private void dump(Function f) {
        if (f == null) return;
        long key = f.getEntryPoint().getOffset();
        if (!done.add(key)) return;
        try {
            DecompileResults res = decomp.decompileFunction(f, 60, mon);
            fp.println("============================================================");
            fp.println(String.format("==== %s @0x%x  size=%d  callers=[%s] ====",
                f.getName(), key, f.getBody().getNumAddresses(), callersOf(f)));
            if (res != null && res.decompileCompleted())
                fp.println(res.getDecompiledFunction().getC());
            else
                fp.println("  DECOMP FAIL");
        } catch (Exception e) { fp.println("  EXC " + e); }
    }

    @Override
    public void run() throws Exception {
        String outPath = OUT_DIR + "jc4_all_functions_decomp.txt";
        new File(OUT_DIR).mkdirs();
        fp = new PrintWriter(new File(outPath), "UTF-8");
        decomp = new DecompInterface();
        decomp.openProgram(currentProgram);
        mon = new ConsoleTaskMonitor();
        FunctionManager fm = currentProgram.getFunctionManager();

        long lo = Long.MAX_VALUE, hi = 0;
        for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
            if (b.isExecute()) {
                lo = Math.min(lo, b.getStart().getOffset());
                hi = Math.max(hi, b.getEnd().getOffset());
            }
        }
        TreeMap<Long, Function> work = new TreeMap<>();
        for (Function f : fm.getFunctions(true)) {
            long off = f.getEntryPoint().getOffset();
            if (off >= lo && off <= hi && !f.isThunk()) work.put(off, f);
        }

        fp.println(String.format("# JC4 ALL export: %d functions", work.size()));
        int i = 0, total = work.size();
        for (Function f : work.values()) {
            dump(f);
            if (++i % 100 == 0) { println("decompiled " + i + "/" + total); fp.flush(); }
            if (mon.isCancelled()) break;
        }
        decomp.dispose();
        fp.close();
        println("done -> " + outPath + "  (" + done.size() + " functions)");
    }
}
