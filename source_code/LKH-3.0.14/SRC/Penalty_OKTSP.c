#include "LKH.h"
#include "Segment.h"

GainType Penalty_OKTSP(void) {
    int Size = 0;
    GainType CostSum = 0;
    int Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;
    Node *N, *NextN = FirstNode;

    N = Depot;
    while (++Size < k && Size < DimensionSaved) {
        NextN = Forward ? SUCC(N) : PREDD(N);
        CostSum += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
        N = Forward ? SUCC(NextN) : PREDD(NextN);
    }
    return CostSum;
}
