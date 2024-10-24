#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>
#include <mpi.h>

#ifndef NOTREE	
#include <treemon.h>
#endif

int main(int argc, char ** argv)
{
	fprintf(stderr, "Program has started\n");
	MPI_Init(&argc, &argv);

	int rank, size;

	MPI_Comm_rank(MPI_COMM_WORLD, &rank);
	MPI_Comm_size(MPI_COMM_WORLD, &size);

	fprintf(stderr, "MPI: %d / %d\n", rank, size);

#ifndef NOTREE
	struct Tbon * tbon = treemon_leaf_init();

		treemon_set_counter(tbon, "rank", rank);
		treemon_set_counter(tbon, "size", size);
		treemon_set_counter(tbon, "pid_modulo5", getpid()%5);
#endif

	uint64_t cnt = 0;

	srand(getpid());



	while(1)
	{
		int rnd = rand()%512;
#ifndef NOTREE	
		treemon_set_counter(tbon,"random", rnd);
		treemon_set_counter(tbon, "counter", cnt);
#endif
		fprintf(stderr, "Counter %d Random %d\n", cnt, rnd);
		sleep(1);
		cnt = cnt + 1;
	}

	MPI_Finalize();

	return 0;
}
