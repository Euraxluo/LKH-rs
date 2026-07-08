#include "LKH.h"
#include "Segment.h"

GainType Penalty_OP(void)
{
    Node *N, *NextN, *PredN;
    GainType Score = 0, PathCost = 0, Cost;
    int Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;

    RouteNodes = 0;
    RouteCost = 0;
    Score = 0;
    N = Depot;
    do {
        Score += N->Score;
        PredN = Forward ? PREDD(N) : SUCC(N);
        Cost = PathCost +
                (C(PredN, Depot) - PredN->Pi - Depot->Pi) / Precision;
        if (Cost <= CostLimit) {
            RouteNodes++;
            RouteScore = -Score;
            RouteCost = Cost;
        }
        NextN = Forward ? SUCC(N) : PREDD(N);
        PathCost += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
        N = Forward ? SUCC(NextN) : PREDD(NextN);
    } while (N != Depot && PathCost <= CostLimit);
    return RouteScore;
}
