#!/usr/bin/env python3
import os
import re
import csv
import filecmp
import subprocess
from statistics import median

def test_and_get_median(args, trials=5):
    """
    Run the provided args on the command line and capture the times provided.
    Returns the median optimization time and running time as a pair.
    """
    pipes = []
    runs  = []
    for i in range(trials):
        cp = subprocess.run(args, stdout=subprocess.PIPE)
        out = cp.stdout.split()

        # program might print extra lines between the timings
        pipe_ns = float(out[0])
        run_ns  = float(out[-1])

        pipes.append(pipe_ns)
        runs.append(run_ns)
    return (median(pipes), median(runs))

def test_ek(ekfile, extra_args=[]):
    """
    Test each grouping of optimizations on the specified kaleidoscope program
    """
    ekbase = re.match(r'.*/(.*)\.ek',ekfile).group(1)
    if not ekbase: # above line errors anyway but this *feels like* error handling
        ekbase = ekfile
    test_groups = {
        'none'      : [],
        'functions' : ['-fargument_promotion','-fbasic_alias_analysis','-ffunction_inlining'],
        'control'   : ['-fcfg_simplification','-faggressive_dce','-fstrip_dead_prototypes'],
        'memory'    : ['-finstruction_combining','-fpromote_memory_to_register'],
        'other'     : ['-find_var_simplify','-floop_vectorize','-freassociate','-fsccp','-fdead_arg_elimination'],
    }
    test_groups['all'] = [flag for group in test_groups for flag in test_groups[group]]
    csvfile = open('results/test_{}.csv'.format(ekbase), 'w')
    writer = csv.writer(csvfile)
    writer.writerow(['optimizations','pipeline time (ns)','run time (ns)'])
    basic_args = './bin/ekcc --jit --time'.split()

    for group in test_groups:
        args = basic_args + test_groups[group] + ['-o', 'output/{}.jit'.format(ekbase)] + [ekfile] + extra_args
        (pipe_ns, run_ns) = test_and_get_median(args)
        writer.writerow([group, pipe_ns, run_ns])

if __name__ == '__main__':
    test_ek('test/test20.ek')
    test_ek('test/hellfile.ek',['4'])
