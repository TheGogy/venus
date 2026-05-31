#!/bin/python

import ast
import sys

import numpy as np
import networkx as nx


def read_file_to_np(fpath: str) -> np.ndarray:
    with open(fpath, "r") as f:
        data_string = f.read()
    return np.asarray(ast.literal_eval(data_string))


def cluster(coacts: np.ndarray) -> np.ndarray:
    N = coacts.shape[0]

    print(f"Clustering {N} activations")

    G1 = nx.Graph()
    for i in range(N):
        for j in range(i + 1, N):
            G1.add_edge(i, j, weight=coacts[i, j])

    pairs = list(nx.max_weight_matching(G1))

    print("Clustering pairs together")

    G2 = nx.Graph()
    for i in range(N / 2):
        for j in range(i + 1, N / 2):
            u, v = pairs[i]
            x, y = pairs[j]
            weight = coacts[u, x] + coacts[u, y] + coacts[v, x] + coacts[v, y]
            G2.add_edge(i, j, weight=weight)

    block_matches = nx.max_weight_matching(G2)

    final_groups = []
    for i, j in block_matches:
        final_groups.extend([*pairs[i], *pairs[j]])

    return np.array(final_groups)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"Usage:\n{sys.argv[0]} coactivations.txt")
        exit(1)

    coacts = read_file_to_np(sys.argv[1])
    perm = cluster(coacts)
    print("Clustered perm:\n\n", perm.tolist())
