import org.apache.commons.math3.random.RandomGenerator;
import org.jamesii.ml3.experiment.init.IInitialStateBuilder;
import org.jamesii.ml3.model.Model;
import org.jamesii.ml3.model.Parameters;
import org.jamesii.ml3.model.agents.AgentDeclaration;
import org.jamesii.ml3.model.agents.IAgent;
import org.jamesii.ml3.model.agents.IAgentFactory;
import org.jamesii.ml3.model.state.IState;
import org.jamesii.ml3.model.state.IStateFactory;
import org.jamesii.ml3.model.values.StringValue;

public class Grid implements IInitialStateBuilder {
    private int size;

    public Grid(int size) {
        this.size = size;
    }

    public IState buildInitialState(Model model, IStateFactory stateFactory, IAgentFactory agentFactory, RandomGenerator rng, Parameters parameters) {
        IState s = stateFactory.create();

        AgentDeclaration agentType = model.getAgentDeclaration("Person");
        IAgent[][] agents = new IAgent[size][size];

        for (int i = 0; i < size; i++) {
            for (int j = 0; j < size; j++) {
                agents[i][j] = agentFactory.createAgent(agentType, 0);
                s.addAgent(agents[i][j]);
            }
        }

        for (int i = 0; i < size; i++) {
            for (int j = 0; j < size; j++) {
                if (i+1 < size) {
                    agents[i][j].addLink("network", agents[i+1][j]);
                }
                if (i-1 >= 0) {
                    agents[i][j].addLink("network", agents[i-1][j]);
                }
                if (j+1 < size) {
                    agents[i][j].addLink("network", agents[i][j+1]);
                }
                if (j-1 >= 0) {
                    agents[i][j].addLink("network", agents[i][j-1]);
                }
                /*
                agents[i][j].addLink("network", agents[(i+1) % size][j]);
                agents[i][j].addLink("network", agents[(i-1+size) % size][j]);
                agents[i][j].addLink("network", agents[i][(j+1) % size]);
                agents[i][j].addLink("network", agents[i][(j-1+size) % size]);
                */
            }
        }

        agents[0][0].setAttributeValue("state", new StringValue("i"));

        return s;
    }
}
