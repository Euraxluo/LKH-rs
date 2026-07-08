"""Run small LKH-rs problems from Python without creating input files."""

import lkh_rs


def main():
    solve_euclidean_tsp()
    solve_distance_matrix_tsp()
    solve_cvrp()
    solve_raw_problem()


def solve_euclidean_tsp():
    parameters = lkh_rs.SearchParameters(seed=1, max_trials=100)
    report = lkh_rs.solve_euclidean_2d(
        [
            (0.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0),
            (1.0, 0.0),
        ],
        parameters,
    )

    print("Euclidean TSP best cost:", report["best_cost"])
    print("Euclidean TSP tour:", report["tour"])


def solve_distance_matrix_tsp():
    problem = lkh_rs.Problem.distance_matrix(
        [
            [0, 1, 2, 1],
            [1, 0, 1, 2],
            [2, 1, 0, 1],
            [1, 2, 1, 0],
        ],
        name="matrix_tsp",
    )
    report = lkh_rs.solve(problem)

    print("Matrix TSP best cost:", report["best_cost"])
    print("Matrix TSP tour:", report["tour"])


def solve_cvrp():
    problem = lkh_rs.Problem.cvrp(
        distance_matrix=[
            [0, 1, 1, 2],
            [1, 0, 2, 1],
            [1, 2, 0, 1],
            [2, 1, 1, 0],
        ],
        demands=[0, 1, 1, 1],
        capacity=3,
        depot=1,
        name="tiny_cvrp",
    )
    parameters = lkh_rs.SearchParameters(max_trials=100)
    report = lkh_rs.solve(problem, parameters)

    print("CVRP best cost:", report["best_cost"])
    print("CVRP best penalty:", report["best_penalty"])
    print("CVRP tour:", report["tour"])


def solve_raw_problem():
    problem = lkh_rs.Problem.raw(
        lkh_rs.ProblemType.ATSP,
        4,
        name="typed_raw_atsp",
        keywords={
            lkh_rs.ProblemKey.EDGE_WEIGHT_TYPE: lkh_rs.EdgeWeightType.EXPLICIT,
            lkh_rs.ProblemKey.EDGE_WEIGHT_FORMAT: lkh_rs.EdgeWeightFormat.FULL_MATRIX,
        },
        sections={
            lkh_rs.ProblemSection.EDGE_WEIGHT: [
                [0, 1, 9, 9],
                [9, 0, 1, 9],
                [9, 9, 0, 1],
                [1, 9, 9, 0],
            ],
        },
    )
    report = lkh_rs.solve(problem)

    print("Raw ATSP best cost:", report["best_cost"])
    print("Raw ATSP tour:", report["tour"])


if __name__ == "__main__":
    main()
