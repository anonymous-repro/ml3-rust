import org.jamesii.ml3.experiment.Experiment;
import org.jamesii.ml3.experiment.Job;
import org.jamesii.ml3.model.agents.IAgent;
import org.jamesii.ml3.model.maps.IValueMap;
import org.jamesii.ml3.model.state.IState;
import org.jamesii.ml3.model.values.IValue;
import org.jamesii.ml3.observation.*;
import org.jamesii.ml3.simulator.factory.NextReactionMethodSimulatorFactory;
import org.jamesii.ml3.simulator.simulators.ISimulator;
import org.jamesii.ml3.simulator.simulators.RuleInstance;
import org.jamesii.ml3.simulator.stop.StopConditions;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.function.Consumer;

public class PerformanceExperiment {
    private Experiment experiment;
    private Consumer<Long> callback;
    private Consumer<Double> ts;
    private Consumer<Integer> ss;
    private Consumer<Integer> is;
    private Consumer<Integer> rs;
    private Map<String, IValue> params;
    private Map<String, IValueMap> maps;
    private boolean observe;

    public PerformanceExperiment(int size, Consumer<Long> callback, Consumer<Double> ts, Consumer<Integer> ss, Consumer<Integer> is, Consumer<Integer> rs, boolean observe) throws IOException {
        this.callback = callback;
        this.ts = ts;
        this.ss = ss;
        this.is = is;
        this.rs = rs;
        this.observe = observe;
        experiment = new Experiment("sir.ml3", 1, StopConditions.NEVER, new Grid(size), new NextReactionMethodSimulatorFactory(), 0.0);
        params = new HashMap<>();
        maps = new HashMap<>();
    }

    public void run() {
        Job job = new Job(experiment, params, maps) {
            @Override
            protected void onSuccess() {
            }

            @Override
            protected void onFailure(Throwable t) {
                System.err.println(t);
            }

            @Override
            protected void instrument(ISimulator simulator) {
                IObserver obs = new FinishObserver();
                obs.registerListener(new WallclockIntervalListener(p -> callback.accept(p.getSecondValue())));
                simulator.addObserver(obs);

                if (observe) {
                    obs = new IntervalObserver(1);
                    obs.registerListener(new IListener() {
                        @Override
                        public void notify(IState state, double now, RuleInstance instance, IAgent agent) {
                            int s = 0, i = 0, r = 0;
                            for (IAgent a : state.getAgentsByType("Person")) {
                                String status = (String) a.getAttributeValue("state").getValue();
                                if (status.equals("s")) s++;
                                else if (status.equals("i")) i++;
                                else if (status.equals("r")) r++;
                            }
                            ts.accept(now);
                            ss.accept(s);
                            is.accept(i);
                            rs.accept(r);
                        }

                        @Override
                        public boolean isActive() {
                            return true;
                        }

                        @Override
                        public void finish() {

                        }
                    });
                    simulator.addObserver(obs);
                }
                /*IObserver obs = new EventTriggeredObserver();
                obs.registerListener(new TimeAndEventCountListener(callback));
                simulator.addObserver(obs);*/
            }
        };

        experiment.addJob(job);
    }

    public void finish() {
        experiment.finish();
    }

    public boolean awaitTermination(long millis) throws InterruptedException {
        return experiment.awaitTermination(millis);
    }
}
