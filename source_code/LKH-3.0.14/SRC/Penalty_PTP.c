#include "LKH.h"
#include "Segment.h"

GainType Penalty_PTP(void)
{
    Node *N, *NextN, *PredN;
    GainType Score = 0, PathCost = 0, Cost;
    int Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;
    GainType P, BestP = PLUS_INFINITY;
    int Nodes = 0;

    Score = 0;
    N = Depot;
    do {
        Nodes++;
        Score += N->Score;
        PredN = Forward ? PREDD(N) : SUCC(N);
        Cost = PathCost +
                (C(PredN, Depot) - PredN->Pi - Depot->Pi) / Precision;
        P = 100 * (Alpha * Cost - Scale * Score);
        if (P < BestP) {
            RouteNodes = Nodes;
            RouteScore = -Score;
            RouteCost = Cost;
            BestP = P;
        }
        NextN = Forward ? SUCC(N) : PREDD(N);
        PathCost += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
        N = Forward ? SUCC(NextN) : PREDD(NextN);
    } while (N != Depot);
    return BestP;
}
