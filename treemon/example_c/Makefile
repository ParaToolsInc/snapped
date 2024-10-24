CC=mpicc

all: main.c
	$(CC) -L../target/debug $^ -o test -ltreemon -Wl,-rpath=$(PWD)/../target/debug -I..
