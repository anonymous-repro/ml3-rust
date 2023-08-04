import org.jamesii.core.util.eventset.*;
import org.jamesii.ml3.experiment.Experiment;
import org.jamesii.ml3.experiment.Job;
import org.jamesii.ml3.model.maps.IValueMap;
import org.jamesii.ml3.model.state.IState;
import org.jamesii.ml3.model.values.IValue;
import org.jamesii.ml3.observation.AbstractObserver;
import org.jamesii.ml3.observation.IObserver;
import org.jamesii.ml3.observation.WallclockIntervalListener;
import org.jamesii.ml3.simulator.evaluate.StatementEvaluationProtocol;
import org.jamesii.ml3.simulator.factory.NextReactionMethodSimulatorFactory;
import org.jamesii.ml3.simulator.simulators.ISimulator;
import org.jamesii.ml3.simulator.simulators.RuleInstance;
import org.jamesii.ml3.simulator.stop.IStopCondition;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import java.util.function.Consumer;

public class InfinieSIRExperiment {
    private Experiment experiment;
    private Consumer<Long> callback;
    private Map<String, IValue> params;
    private Map<String, IValueMap> maps;
    int nIgnore, nMeasure;

    public InfinieSIRExperiment(int size, Consumer<Long> callback, int nIgnore, int nMeasure) throws IOException {
        this.callback = callback;
        this.nIgnore = nIgnore;
        this.nMeasure = nMeasure;
        NextReactionMethodSimulatorFactory f = new NextReactionMethodSimulatorFactory(); //new SimpleEventQueueFactory<>());
        experiment = new Experiment("infinite_sir.ml3", 1, new StopAfterEvents(nIgnore+nMeasure), new Grid(size), f, 0.0);
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
                IObserver obs = new AbstractObserver() {
                    int count = 0;

                    @Override
                    public void updateBefore(IState state, double now, double timeOfNextEvent, RuleInstance instance) {}

                    @Override
                    public void updateAfter(IState state, double now, RuleInstance instance, StatementEvaluationProtocol sep) {
                        if (++count == nIgnore) {
                            notifyListeners(state, now, instance, null);
                        } else if (count == nIgnore + nMeasure) {
                            notifyListeners(state, now, instance, null);
                        }
                    }

                    @Override
                    public void finish(IState state, double now) {
                        finishListeners();
                    }
                };
                //obs = new FinishObserver();
                obs.registerListener(new WallclockIntervalListener(p -> callback.accept(p.getSecondValue())));
                simulator.addObserver(obs);
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

    private class StopAfterEvents implements IStopCondition {
        int n;
        int count;

        public StopAfterEvents(int n) {
            this.n = n;
        }

        @Override
        public boolean test(IState state, double time) {
            return ++count > n;
        }
    }
}
