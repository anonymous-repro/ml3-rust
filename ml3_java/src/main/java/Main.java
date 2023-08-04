import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;

public class Main {
    public static void main(String[] args) throws IOException, InterruptedException {

        for (int i = 0; i < 100; i++) {
            run(32, false);
        }

        run(32, true);


        nRuns(32, 10);
        nRuns(100, 10);
        //nRuns(316, 10);
    }

    private static void nRuns(int size, int runs) throws IOException, InterruptedException {
        double sum = 0;
        for (int i = 0; i < runs; i++) {
            sum += run(size, false);
        }
        System.out.println(size + "x" + size + ": " + Math.round(sum / runs) + "/s");
    }

    private static double run(int size, boolean output) throws IOException, InterruptedException {
        final long[] runtime = new long[1];
        final long[] events = new long[1];
        final ArrayList<Double> ts = new ArrayList<>();
        final ArrayList<Integer> ss = new ArrayList<>();
        final ArrayList<Integer> is = new ArrayList<>();
        final ArrayList<Integer> rs = new ArrayList<>();

        PerformanceExperiment exp;
        exp = new PerformanceExperiment(size, t -> runtime[0] = t, ts::add, ss::add, is::add, rs::add, output);

        exp.run();
        exp.finish();

        while (!exp.awaitTermination(60000)) {
            System.out.println("running...");
        }

        events[0] = 2 * size * size - 1;

        if (output) {
            writeCSV("out_" + size + "x" + size + ".csv", ts, ss, is, rs);
        }

        return events[0] / (runtime[0] * 1e-9);
    }

    private static void writeCSV(String file, List<Double> ts, List<Integer> ss, List<Integer> is, List<Integer> rs) throws IOException {
        PrintWriter out = new PrintWriter(new FileWriter(file));
        out.println("t, s, i, r");
        for (int i = 0; i < ts.size(); i++) {
            out.println(ts.get(i) + ", " + ss.get(i) + ", " + is.get(i) + ", " + rs.get(i));
        }
        out.flush();
        out.close();
    }
}
