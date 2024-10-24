/**
 * @file treemon.h
 * @brief TreeMon data structure and functions.
 *
 * This file provides a tree-based data structure (Tbon) for efficient storage
 * and retrieval of counter values.
 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * @struct Tbon
 * @brief The Tbon data structure, representing a node in the TreeMon.
 */
typedef struct Tbon Tbon;

/**
 * @brief Initializes the root node of the TreeMon.
 *
 * This function creates and returns a new root node for the TreeMon. It is
 * the starting point for building the tree structure.
 *
 * @return A pointer to the newly created root node, or NULL on failure.
 */
struct Tbon *treemon_root_init(void);

/**
 * @brief Initializes a leaf node in the TreeMon.
 *
 * This function creates and returns a new leaf node that can be inserted
 * into the TreeMon structure. Leaf nodes store counter values.
 *
 * @return A pointer to the newly created leaf node, or NULL on failure.
 */
struct Tbon *treemon_leaf_init(void);

/**
 * @brief Sets the value of a counter in the TreeMon.
 *
 * This function searches for a counter with the specified name (cnt) and
 * sets its value to the provided uint64_t value. If the counter does not exist,
 * it will be created.
 *
 * @param tbon A pointer to the root node or any other node of the TreeMon.
 * @param cnt The name of the counter to set.
 * @param value The new value for the counter.
 *
 * @return 0 on success, non-zero error code otherwise.
 */
int32_t treemon_set_counter(struct Tbon *tbon, const char *cnt, uint64_t value);
