import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.pylab as pl
import json
import matplotlib as mpl

import json

import glob

def plot_topology(name):
    with open(name) as json_file:
        data = json.load(json_file)
    node_x = np.array(list(map(lambda x:x["x"],data["nodes"])))
    node_y = np.array(list(map(lambda x:x["y"],data["nodes"])))
    node_type = list(map(lambda x:x["loc_type"],data["nodes"]))
    plt.plot([],[],"-",color="grey",label="fast link")
    plt.plot([],[],"--",color="grey",label="slow link")
    #print(data)
    for l in data["links"]:
        c = l["connects"]
        xs = [node_x[c[0]],node_x[c[1]]]
        ys = [node_y[c[0]],node_y[c[1]]]
        if l["link_type"] == 'Fast':
            ls = "-"
        else:
            #print(l["link_type"])
            ls = "--"

        plt.plot(xs,ys,ls=ls)
    print(set(node_type))
    num_entry = node_type.count("Entry")
    num_exit = node_type.count("Exit")
    plt.scatter(node_x[num_entry:-num_exit],node_y[num_entry:-num_exit],marker="o",label="Node")
    plt.scatter(node_x[:num_entry],node_y[:num_entry],marker=">",label="Entry")
    plt.scatter(node_x[-num_exit:],node_y[-num_exit:],marker="<",label="Exit")
    plt.xlim(xmax=1.3)
    plt.legend()
    plt.savefig("topo.pdf")

def plot_output():
    data = pd.read_csv("output.csv")


    plt.plot(data["time"],data["Person { status: Infected }"],label="infected")
    plt.plot(data["time"],data["Person { status: Recovered }"],label="recovered")
    plt.plot(data["time"],data["Person { status: Susceptible }"],label="susceptible")

    # used for the reference file by java ml3
    data = pd.read_csv("java_reference.csv")


    plt.plot(data["t"],data[" i"],'--',label="infected")
    plt.plot(data["t"],data[" r"],'--',label="recovered")
    plt.plot(data["t"],data[" s"],'--',label="susceptible")

    plt.xlim(left=0,right=60)
    plt.ylabel("counts")
    plt.xlabel("time")

    plt.legend()
    #plt.show()
    plt.savefig("plot.svg")


def plot_routes():
    max_city=100
    with open('routes_output_high_error.json', 'r') as f:
        data = json.load(f)
    #print(data.keys())
    time = np.array(data["time_mean"])
    plt.xlim(xmax=100)
    plt.xlabel("Time [a.u.]")
    plt.ylabel("Information Deviation")

    colors = pl.cm.plasma(np.sqrt(np.linspace(0,1,max_city)))
    #mpl.colorbar.ColorbarBase(plt.gca(),cmap=pl.cm.plasma,po)
    sm = plt.cm.ScalarMappable(cmap=pl.cm.plasma)
    #cbar = plt.colorbar(sm)
    #cbar.ax.set_ylabel("x location of node")
    #plt.title("Information Deviation in 100 City model")
    for n in range(max_city):
        if n % 3 != 0:
            continue
        name = "city_"+str(n)+"_accuracy"
        d = np.array(data[name+"_mean"],dtype=np.float)
        d_err = np.array(data[name+ "_stddev"])
        stencil = d==d
        #plt.errorbar(time[stencil],d[stencil],yerr=d_err[stencil],errorevery=10,label=name,color=colors[n])
        plt.plot(time[stencil],d[stencil],label=name,color=colors[n])
    #//plt.colorbar()
    #plt.legend()
    plt.ylim(ymin=0)
    #plt.clf()
    #plt.errorbar(time, data["active migrants_mean"],yerr=data["active migrants_stddev"],errorevery=100)
    #print(data["time_mean"])
    #plt.show()
    plt.gcf().set_size_inches(3.5,3.5)
    plt.tight_layout()
    plt.savefig("accuracy_high_error.pdf")
    plt.clf()

    plt.xlabel("Time [a.u.]")
    plt.ylabel("Visits/time [a.u.]")
    colors = pl.cm.plasma(np.sqrt(np.linspace(0,1,max_city)))
    #mpl.colorbar.ColorbarBase(plt.gca(),cmap=pl.cm.plasma,po)
    sm = plt.cm.ScalarMappable(cmap=pl.cm.plasma)
    cbar = plt.colorbar(sm)
    cbar.ax.set_ylabel("x location of node")
    for n in range(max_city):
        name = "city_"+str(n)+"_visits"
        arrive = np.array(data[name+"_mean"],dtype=np.float)
        arrive = arrive - np.roll(arrive,1)
        arrive = arrive[1:-1]
        plt.plot(time[1:-1],arrive,color=colors[n],lw=0.1)
        d_err = np.array(data[name+ "_stddev"])
        stencil = arrive==arrive
        #plt.errorbar(time[stencil],d[stencil],yerr=d_err[stencil],errorevery=10,label=name,color=colors[n])
        #plt.plot(time[stencil],arrive[stencil],label=name,color=colors[n])
    #//plt.colorbar()
    #plt.legend()
    #plt.ylim(ymin=0)
    #plt.clf()
    #plt.errorbar(time, data["active migrants_mean"],yerr=data["active migrants_stddev"],errorevery=100)
    #print(data["time_mean"])
    #plt.show()
    plt.savefig("arrivals.pdf")
    plt.clf()

    arrive = data["city_3_visits_mean"]
    plt.ylabel("arrivels per time")
    arrive = arrive - np.roll(arrive,1)
    arrive = arrive[1:-1]
    plt.plot(time[1:-1],arrive)
    plt.savefig("arrivals_3.pdf")

def read_perf_data(path):
    x = []
    val = []
    lower = []
    upper = []
    #for i in range(1,5000):
    globi = glob.glob(path+"/*N=*/new/estimates.json")
    globi = sorted(globi)


    for file in globi:
        i = int(file.split("N=")[1].split("/")[0])
        x.append(i)
        with open(file) as f:
            res = json.load(f)
            val.append(res["mean"]["point_estimate"])
            lower.append(res["mean"]["confidence_interval"]["lower_bound"])
            upper.append(res["mean"]["confidence_interval"]["upper_bound"])

    x = np.array(x)

    np.savetxt("last_data.csv",[x,val])
    thrpt = 1/np.array(val)*1e9
    thrpt_up = 1 / np.array(upper) * 1e9
    thrpt_lo = 1 / np.array(lower) * 1e9

    return (x,thrpt,thrpt_lo,thrpt_up)

def plot_performance():
    plt.vlines(601,10,10000000,linestyles="dotted",label="Size from experiment")

    (x,thrpt,_,_) =read_perf_data("target/criterion/Infinite Sir")
    plt.plot(x,thrpt,'-*',label="rust linked lives")

    (x,thrpt,_,_) =read_perf_data("../custom_sir/target/criterion/seq")
    plt.plot(x,thrpt,'-*',label="custom Lin")

    (x,thrpt,_,_) =read_perf_data("../custom_sir/target/criterion/tree")
    plt.plot(x,thrpt,'-*',label="custom tree")


    java = np.genfromtxt('java_perf.csv',delimiter=',')
    plt.plot(java[:,0],java[:,1],label="ml3-java")

    #(x,thrpt,_,_) =read_perf_data("../custom_sir/target/criterion/seq")
    #plt.plot(x,thrpt,'-*',label="custom Lin")
    #plt.plot(julia_x,julia_thrp,'-*',label="julia ABM Template")
    #plt.plot(java_x,java_thrp,'-*',label="java ml3")

    with open('../custom_sir/netlogo/netlogo_perf.json') as f:
        netlogo = json.load(f)

    netlogo_x = np.array(netlogo["x"])

    netlogo_x = netlogo_x*netlogo_x
    plt.errorbar(netlogo_x,netlogo["netlogo_thrp32"],np.array(netlogo["netlogo_thrp32_err"])*1e-3,label="Netlogo $\Delta$t = 32")
    plt.errorbar(netlogo_x,netlogo["netlogo_thrp1"],netlogo["netlogo_thrp1_err"],label="Netlogo $\Delta$t = 1")


    plt.legend(loc="right")
    plt.xlim(xmin=2,xmax=1e11)
    #plt.plot(x, thrpt_up)
    #plt.plot(x, thrpt_lo)

    plt.yscale('log')
    plt.xscale('log')

    plt.xlabel("Number of agents")
    plt.ylabel("throughput [reactions/s]")
    plt.savefig("performance.pdf")
    plt.savefig("performance.svg")
    #plt.show()


def num_to_reac_name(num):
    if num == 0:
        return "cost stay"
    if num == 1:
        return "explore"
    if num == 2:
        return "mingle"
    if num == 3:
        return "communication"
    if num == 4:
        return "decide new location"
    if num == 5:
        return "arrive"
    if num == 6:
        return "handle departures"
    return "{}".format(num)
def plot_routes_performance():
    with open('routes_output.json', 'r') as f:
        data = json.load(f)
    sim_time = data["time_mean"]
    times_all = []
    times_per_count = []
    counts_all = []
    for i in range(7):
        times = np.array(data["reac_time_spend_exec_{}_mean".format(i)])
        countns = np.array(data["reac_counts_{}_mean".format(i)])
        times_all.append(times)
        times_per_count.append(times/countns)
        counts_all.append(countns)

    labels = []
    for x in range(len(times)):
        labels.append(num_to_reac_name(x))

    plt.stackplot(sim_time,times_all,labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.yticks([],[])
    plt.ylabel("total CPU-time spend on transition")
    plt.legend(loc="upper left")
    plt.savefig("routes_time_abs.pdf")
    plt.clf()

    plt.stackplot(data["time_mean"],np.array(times_per_count)*1e3,labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.ylabel("CPU-time per transition execution [ms]")
    plt.legend(loc="upper left")
    plt.savefig("routes_time_rel.pdf")
    plt.clf()

    plt.stackplot(data["time_mean"],np.array(counts_all)/(sim_time[1]-sim_time[0]),labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.ylabel("transition counts per unit of simulation time")
    plt.legend(loc="upper left")
    plt.savefig("routes_counts.pdf")
    plt.clf()


#plot_routes_performance()

plot_routes()
#plot_topology("topo_100_city.json")