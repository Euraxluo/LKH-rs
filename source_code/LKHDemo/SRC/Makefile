TREE_TYPE = TWO_LEVEL_TREE
# TREE_TYPE = THREE_LEVEL_TREE
# TREE_TYPE = ONE_LEVEL_TREE

# CC = gcc
IDIR = INCLUDE
ODIR = OBJ
CFLAGS = -O3 -Wall -I$(IDIR) -D$(TREE_TYPE) -g -fcommon

_DEPS = demo.h demo2.h

DEPS = $(patsubst %,$(IDIR)/%,$(_DEPS))

_OBJ = demo.o demo2.o LKHmain.o  GetTime.o

OBJ = $(patsubst %,$(ODIR)/%,$(_OBJ))

$(ODIR)/%.o: %.c $(DEPS)
	$(CC) -c -o $@ $< $(CFLAGS)

.PHONY: 
	all clean

all:
	$(MAKE) LKH

LKH: $(OBJ) $(DEPS)
	$(CC) -o ../LKH $(OBJ) $(CFLAGS) -lm

clean:
	/bin/rm -f $(ODIR)/*.o ../LKH *~ ._* $(IDIR)/*~ $(IDIR)/._* 

