#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <treemon.h>
#include <unistd.h>
#include <mpi.h>

int main(int argc, char ** argv)
{
	MPI_Init(&argc, &argv);

	int rank, size;

	MPI_Comm_rank(MPI_COMM_WORLD, &rank);
	MPI_Comm_size(MPI_COMM_WORLD, &size);

	printf("%d / %d\n", rank, size);

	struct Tbon * tbon = treemon_leaf_init();

		treemon_set_counter(tbon, "rank", rank);
		treemon_set_counter(tbon, "size", size);

	uint64_t cnt = 0;

	srand(getpid());

		treemon_set_counter(tbon, "pid_modulo5", getpid()%5);


	while(1)
	{
		treemon_set_counter(tbon, "counter", cnt);
		treemon_set_counter(tbon,"random", rand()%512);

		sleep(1);
		cnt = cnt + 1;
	}

	MPI_Finalize();

	return 0;
}
