CC=mpicc

all: test test_no_link

test_no_link: main.c
	$(CC) -DNOTREE  $^ -o $@ 

test: main.c
	$(CC) -L../target/debug $^ -o $@ -ltreemon -Wl,-rpath=$(PWD)/../target/debug -I..


test_release: main.c
	$(CC) -L../target/release $^ -o $@ -ltreemon -Wl,-rpath=$(PWD)/../target/release -I..
