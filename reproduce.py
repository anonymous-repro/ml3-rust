import fileinput
import sys
import subprocess
import os
import json
import matplotlib.pyplot as plt
import matplotlib.pylab as pl
import numpy as np
import tqdm
import time
from scipy.stats import sem
import pandas as pd


def replace_in_file(file,searchExp,replaceExp):
    for line in fileinput.input(file, inplace=1):
        if searchExp in line:
            line = replaceExp + "\n"
        sys.stdout.write(line)

def generate_data_routes(name,err,reps,until):
    replace_in_file("examples/routes.rs", "// stochastic error when transmitting information",f"const error: f64 = {err}; // stochastic error when transmitting information")
    subprocess.check_output(f"cargo run --release --example routes {reps} {until}",shell=True)
    os.rename("routes_output.json",f"routes_output_{name}.json")

def plot_routes(postfix):
    max_city=100
    with open(f'routes_output_{postfix}.json', 'r') as f:
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
        d = np.array(data[name+"_mean"],dtype=float)
        d_err = np.array(data[name+ "_stddev"])
        stencil = d==d
        plt.plot(time[stencil],d[stencil],label=name,color=colors[n])
    plt.ylim(ymin=0)

    plt.gcf().set_size_inches(3.5,3.5)
    plt.tight_layout()
    plt.savefig(f"accuracy_{postfix}.pdf")
    plt.clf()

def plot_routes_performance(postfix):

    def smooth(x):
        #return x
        new_x =  np.diff(np.cumsum(x)[::3])/3
        return new_x
        return (x + np.roll(x,+1))[1::2]

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

    with open(f'routes_output_{postfix}.json', 'r') as f:
        data = json.load(f)
    sim_time = smooth(data["time_mean"])
    times_all = []
    times_per_count = []
    counts_all = []
    for i in range(7):
        times = np.array(smooth(data["reac_time_spend_exec_{}_mean".format(i)]))
        countns = np.array(smooth(data["reac_counts_{}_mean".format(i)]))
        times_all.append(times)
        times_per_count.append(times/countns)
        counts_all.append(countns)



    labels = []
    for x in range(len(times)):
        labels.append(num_to_reac_name(x))

    plt.xlim(xmin=0,xmax=100)

    plt.stackplot(sim_time,times_all,labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.yticks([],[])
    plt.ylabel("total CPU-time spend on transition")
    #plt.legend(loc="upper left")
    plt.gcf().set_size_inches(3.5,3.5)
    plt.tight_layout()
    plt.savefig("routes_perf_time_abs.pdf")
    plt.clf()

    plt.xlim(xmin=0,xmax=100)
    plt.stackplot(sim_time,np.array(times_per_count)*1e3,labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.ylabel("CPU-time per transition execution [ms]")
    #plt.legend(loc="upper left")
    plt.gcf().set_size_inches(3.5,3.5)
    plt.tight_layout()
    plt.savefig("routes_perf_time_rel.pdf")
    plt.clf()

    plt.xlim(xmin=0,xmax=100)
    plt.stackplot(sim_time,np.array(counts_all)/(sim_time[1]-sim_time[0]),labels=labels)
    plt.xlabel("simulation time [a.u.]")
    plt.ylabel("transition counts per unit of simulation time")
    #plt.legend(loc="upper left")
    plt.gcf().set_size_inches(3.5,3.5)
    plt.tight_layout()
    plt.savefig("routes_perf_counts.pdf")
    plt.clf()

    plt.stackplot([],list(map(lambda _ : [],range(7))),labels=labels)
    plt.axis('off')
    plt.legend(ncol=7)
    plt.gcf().set_size_inches(10.5,0.5)
    plt.tight_layout()
    plt.savefig("routes_perf_legend.pdf")
    plt.clf()

def measuere_netlogo():
    # you may need to change the path to netlogo here
    netlogo = "NetLogo_6/netlogo-headless.sh"
    reps = 5
    ys1 = []
    y_sems1 = []
    ys32 = []
    y_sems32 = []
    num_steps = np.array([1.75, 2.55, 55.45, 1224.4, 10386.15, 45112.55, 182452.85, 736814.0, 2959149.45, 11870687.4])
    xvals = np.array([2, 4, 8, 16, 32, 64, 128, 256, 512,1024])



    for x in xvals:
        if x > 100 and reps > 3:
            reps = 3
        print("running {}x{}".format(x,x))
        overheads = np.array([])
        for _ in tqdm.tqdm(range(reps)):
            start = time.time()
            subprocess.check_output(netlogo + " --model netlogo/sir.nlogo --experiment Perf_single --min-pxcor 0 --max-pxcor {} --min-pycor 0 --max-pycor {}".format(x,x),shell=True)
            overhead = (time.time() - start)/reps
            overheads = np.append(overheads,overhead)
        print('Overhead: ',overheads)

        for var in [32,1]:
            times = np.array([])
            for _ in tqdm.tqdm(range(reps)):
                start = time.time()
                subprocess.check_output(netlogo + " --model netlogo/sir.nlogo --experiment Perf{} --min-pxcor 0 --max-pxcor {} --min-pycor 0 --max-pycor {}".format(var,x,x),shell=True)
                time_32 = (time.time() - start)/reps
                times = np.append(times,time_32)
            thrp = np.median(times)-np.median(overheads)
            thrp_sem = sem(times) + sem(overheads)
            if var == 1:
                y_sems1 = np.append(y_sems1,thrp_sem)
                ys1 = np.append(ys1,thrp)
                print('time_1: ',times, " -> ",thrp)
                outp = (num_steps[:len(ys1)]-1)/ys1
                errs = outp * (y_sems1/ys1)
                print("netlogo_thrp1 =  ", outp.tolist() , "\nnetlogo_thrp1_err =  ",errs.tolist())
            elif var == 32:
                y_sems32 = np.append(y_sems32,thrp_sem)
                ys32 = np.append(ys32,thrp)
                print('time_32: ',times, " -> ",thrp)
                outp = (num_steps[:len(ys32)]-1)/ys32
                errs = outp * (y_sems32/ys32)
                print("netlogo_thrp32 =  ", outp.tolist() , "\nnetlogo_thrp32_err =  ",errs.tolist())
        results = {"xvals" : xvals.tolist(),"ys1":ys1.tolist(),"y_sems1":y_sems1.tolist(),"ys32":ys32.tolist(),"y_sems32":y_sems32.tolist()}
        with open("netlogo_timings.json", "w") as outfile:
            outfile.write(json.dumps(results))

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


def plot_reference_results():
    subprocess.check_output("cargo run  --manifest-path custom_sir/Cargo.toml --release --example make_ref 10000", shell=True)
    my_data = np.genfromtxt('reference.csv', delimiter=',')
    plt.plot(my_data[:,0],my_data[:,1])
    plt.savefig("reference_results.pdf")
    plt.show()

def figures_5_and_6():
    for setups in [(4,10.),(24,30.),(128,50.),(500,100.)]:
        (reps,endtime) = setups
        for faktor in [1/5,1/2,1,2,5]:
            name = f"err_{faktor}"
            generate_data_routes(name,faktor*0.1,reps,endtime)
            plot_routes(name)
        plot_routes_performance("err_1")

def figure_4():
    plot_topology("topo_100_city.json")

def figure_7():
    plot_reference_results()

plot_reference_results()
#measuere_netlogo()

