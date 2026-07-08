#include "LKH.h"
#include "Segment.h"

static int PenaltySum = -1;
static int ScoreSum = -1;
static int ScoreLimit = 0;

GainType Penalty_PC_TSP(void)
{
    Node *N, *NextN, *PredN;
    GainType Score = 0, PathCost = 0, Cost = 0, Penalty = 0;
    int Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;
    GainType P, BestP = PLUS_INFINITY;
    int Nodes = 0;

    if (PenaltySum == -1) {
        ScoreSum = PenaltySum = 0;
        N = Depot;
        do {
            ScoreSum += N->Score;
            PenaltySum += N->Penalty;
        } while ((N = SUCC(N)) != Depot);
        ScoreLimit = Alpha * ScoreSum;
    }

    Score = 0;
    Penalty = PenaltySum;
    N = Depot;
    do {
        Nodes++;
        Score += N->Score;
        Penalty -= N->Penalty;
        PredN = Forward ? PREDD(N) : SUCC(N);
        Cost = PathCost +
                (C(PredN, Depot) - PredN->Pi - Depot->Pi) / Precision;
        P = Cost + Scale * Penalty;
        if (P < BestP && Score >= ScoreLimit) {
            RouteNodes = Nodes;
            RouteScore = Score;
            RouteCost = Cost;
            BestP = P;
        }
        NextN = Forward ? SUCC(N) : PREDD(N);
        PathCost += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
        N = Forward ? SUCC(NextN) : PREDD(NextN);
    } while (N != Depot);
    return BestP;
}
