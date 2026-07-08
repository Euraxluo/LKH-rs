#include "LKH.h"
#include "Segment.h"

/* Original code by Ke Xu, University of Southern California */
/* Slightly revised by Keld Helsgaun  */

#define Pen1 100000
#define Pen2 100000
#define Pen3 100000
#define DroneLaunch 1           // 1 minute
#define DroneRetrieve 1         // 1 minute
#define Dis_Scale 10

static int COLOR_COUNT = -1;
static int *ColorVisitsCount;
static GainType *ColorVisitTimes;
static int *RealArrival;
static GainType *L_times;

GainType Penalty_TSPMD(void)
{
    Node *N, *NextN, *PrevN = 0;
    GainType P = 0, LastRouteCost;
    int Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;
    int Load = Drones, ColorsVisitedTwice = 0;

    if (COLOR_COUNT == -1) {
        N = FirstNode;
        do
            if (N->Color > COLOR_COUNT)
                COLOR_COUNT = N->Color;
        while ((N = SUCC(N)) != FirstNode);
        ColorVisitsCount = (int *) malloc((COLOR_COUNT + 1) * sizeof(int));
        ColorVisitTimes = (GainType *)
            malloc((COLOR_COUNT + 1) * sizeof(GainType));
        RealArrival = (int *) malloc((COLOR_COUNT + 1) * sizeof(int));
        if (Endurance > 0)
            L_times =
                (GainType *) malloc((COLOR_COUNT + 1) * sizeof(GainType));
    }
    memset(ColorVisitsCount, 0, (COLOR_COUNT + 1) * sizeof(int));
    memset(ColorVisitTimes, 0, (COLOR_COUNT + 1) * sizeof(GainType));
    memset(RealArrival, 0, (COLOR_COUNT + 1) * sizeof(int));
    if (Endurance > 0)
        memset(L_times, 0, (COLOR_COUNT + 1) * sizeof(GainType));

    RouteNodes = 0;
    LastRouteCost = RouteCost = 0;
    N = Depot;
    do {
        RouteNodes++;
        if (N->Id <= Dim && N != Depot) {
            int ColorIndex = N->Color, PhysicalIndex = N->DraftLimit;
            NextN = Forward ? SUCC(SUCC(N)) : PREDD(PREDD(N));
            PrevN = Forward ? PREDD(PREDD(N)) : SUCC(SUCC(N));

            if (PhysicalIndex != 0 && RealArrival[PhysicalIndex] >= 1)
                P += Pen1;
            if (N->ServiceTime == 0 && ColorVisitsCount[ColorIndex] < 2) {
                ColorVisitsCount[ColorIndex] = 2;
                ColorsVisitedTwice++;
            }
            if (N->ServiceTime != 0 && ColorVisitsCount[ColorIndex] == 1) {
                if (PhysicalIndex == 0 &&
                    ColorsVisitedTwice < COLOR_COUNT - Drones &&
                    NextN->DepotId == 0)
                    P += Pen1;
                RealArrival[ColorIndex] = 1;    /* no revisit 2 */
                ColorVisitsCount[ColorIndex] = 2;
                ColorsVisitedTwice++;
                Load++;
                ColorVisitTimes[ColorIndex] += N->ServiceTime;
                if (Endurance > 0) {
                    if (ColorVisitTimes[ColorIndex] > RouteCost)
                        // truck arrives earlier than drone
                        RouteCost =
                            ColorVisitTimes[ColorIndex] +
                            Dis_Scale * DroneRetrieve;
                    else
                        RouteCost += Dis_Scale * DroneRetrieve;
                    if (RouteCost - L_times[ColorIndex] >
                        Dis_Scale * Endurance)
                        P += Pen2 * ((RouteCost - L_times[ColorIndex]) -
                                     Dis_Scale * Endurance);
                } else if (ColorVisitTimes[ColorIndex] >= RouteCost)
                    RouteCost = ColorVisitTimes[ColorIndex];
            }
            if (N->ServiceTime != 0 && ColorVisitsCount[ColorIndex] == 0) {
                if (PhysicalIndex == 0 && PrevN->DraftLimit != 0)
                    P += Pen1;
                ColorVisitsCount[ColorIndex] = 1;
                Load--;
                if (Endurance > 0) {
                    if (PhysicalIndex != 0)
                        RouteCost += Dis_Scale * DroneLaunch;
                    L_times[ColorIndex] = RouteCost;
                }
                ColorVisitTimes[ColorIndex] = RouteCost + N->ServiceTime;
            }
            if (PhysicalIndex != 0 && NextN->DraftLimit != PhysicalIndex)
                RealArrival[PhysicalIndex]++;
            if (Load > Drones)
                P += Pen2 * (Load - Drones);
            if (Load < 0)
                P -= Pen2 * Load;
            if (P > CurrentPenalty ||
                (P == CurrentPenalty && CurrentGain <= 0))
                return CurrentPenalty + (CurrentGain > 0);
        }
        NextN = Forward ? SUCC(N) : PREDD(N);
        LastRouteCost = RouteCost;
        RouteCost += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
        N = Forward ? SUCC(NextN) : PREDD(NextN);
    } while (N != Depot && ColorsVisitedTwice < COLOR_COUNT);
    if (ColorsVisitedTwice < COLOR_COUNT)
        eprintf("Penalty_TSPMD: ColorsVisitedTwice < COLOR_COUNT");
    /* Go back to the depot */
    PrevN = Forward ? SUCC(PrevN) : PREDD(PrevN);
    RouteCost = LastRouteCost +
        (C(Depot, PrevN) - Depot->Pi - PrevN->Pi) / Precision;
    P += RouteCost;
    P += abs(Load - Drones) * Pen3;
    return P;
}
