#include "LKH.h"
#include "Segment.h"

static GainType GetRoutes();

int **R;
int *RSize;
int Clients;
GainType *RScore;
GainType *RCost;

void TOP_WriteSolution(char *FileName, GainType Cost)
{
    FILE *SolutionFile;
    char *FullFileName;
    time_t Now;
    int i, j;

    if (FileName == 0)
        return;
    FullFileName = FullName(FileName, Cost);
    Now = time(&Now);
    if (TraceLevel >= 1)
        printff("Writing TOP_SOLUTION_FILE: \"%s\" ... ",
                FullFileName);
    SolutionFile = fopen(FullFileName, "w");
    fprintf(SolutionFile, "Found by LKH-3 [Keld Helsgaun] %s", ctime(&Now));
    R = (int **) malloc(Salesmen * sizeof(int *));
    for (i = 0; i < Salesmen; i++)
        R[i] = calloc(DimensionSaved - Salesmen + 1, sizeof(int));
    RSize = (int *) malloc(Salesmen * sizeof(int));
    RScore = (GainType *) malloc(Salesmen * sizeof(GainType));
    RCost = (GainType *) malloc(Salesmen * sizeof(GainType));
    fprintf(SolutionFile, "Profit = "GainFormat"\n", -GetRoutes());
    fprintf(SolutionFile, "Clients = %d\n", Clients);
    fprintf(SolutionFile, "Cost limit = %0.2f\n", 
            CostLimit / Scale);
    for (i = 0; i < Salesmen; i++) {
        fprintf(SolutionFile,
                "Route %d, Clients = %d, Profit = "GainFormat", Cost = %0.2f\n", 
                i + 1, RSize[i], RScore[i], 1.0 * RCost[i] / Scale);
        fprintf(SolutionFile, "%d ", 1);
        for (j = 0; j < RSize[i]; j++)
            fprintf(SolutionFile, "%d ", R[i][j]);
        fprintf(SolutionFile, "%d\n", DimensionSaved - Salesmen + 1);
    }
    fclose(SolutionFile);
    if (TraceLevel >= 1)
        printff("done\n");
}

static GainType GetRoutes()
{
    Node *EndNode = &NodeSet[Dim];
    GainType TotalScore = 0;
    int i, Forward = SUCC(Depot)->Id != Depot->Id + DimensionSaved;

    for (i = 0; i < Salesmen; i++) {
        Node *N = i == 0 ? Depot : &NodeSet[Dim + i];
        GainType Score = 0, RouteScore = 0;
        GainType PathCost = 0, Cost;
        int Size = 0;
        do {
            Node *PredN = Forward ? PREDD(N) : SUCC(N), *NextN;
            Score += N->Score;
            Cost = PathCost +
                    (C(PredN, EndNode) - PredN->Pi - EndNode->Pi) / Precision;
            if (!N->DepotId) {
                R[i][Size] = N->Id;
                Size++;
            }
            if (Cost <= CostLimit) {
                RSize[i] = Size;
                RScore[i] = Score;
                RCost[i] = Cost;
            }
            NextN = Forward ? SUCC(N) : PREDD(N);
            if (NextN - DimensionSaved == EndNode)
                NextN = Forward ? SUCC(EndNode) : PREDD(EndNode);
            PathCost += (C(N, NextN) - N->Pi - NextN->Pi) / Precision;
            N = Forward ? SUCC(NextN) : PREDD(NextN);
        } while (N->DepotId == 0 && PathCost <= CostLimit);
        Clients += RSize[i];
        TotalScore += RScore[i];
    }
    return -TotalScore;
}

