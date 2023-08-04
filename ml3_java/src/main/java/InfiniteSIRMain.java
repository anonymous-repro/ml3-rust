import java.io.IOException;
import java.util.ArrayList;

public class InfiniteSIRMain {
    public static void main(String[] args) throws IOException, InterruptedException {
        int nIgnore = 10000;
        int nMeasure = 10000;
        int reps = 1;

        System.out.print(run(4, nIgnore, nMeasure, reps));
        System.out.print(", " + run(8, nIgnore, nMeasure, reps));
        System.out.print(", " + run(16, nIgnore, nMeasure, reps));
        System.out.print(", " + run(32, nIgnore, nMeasure, reps));
        System.out.print(", " + run(64, nIgnore, nMeasure, reps));
        System.out.print(", " + run(128, nIgnore, nMeasure, reps));
    }

    private static double run(int size, int nIgnore, int nMeasure, int reps) throws IOException, InterruptedException {
        final ArrayList<Long> runtimes = new ArrayList<>();

        for (int i = 0; i < reps; i++) {
            InfinieSIRExperiment exp;
            exp = new InfinieSIRExperiment(size, runtimes::add, nIgnore, nMeasure);

            exp.run();
            exp.finish();

            while (!exp.awaitTermination(60000)) {
            }
        }

        double avg = 0;
        for (int i = 0; i < reps; i++) {
            avg += (runtimes.get(2*i+1) * 1e-9) / reps;
        }

        return nMeasure / avg;
    }
}
