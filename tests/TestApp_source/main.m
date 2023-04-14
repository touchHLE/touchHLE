/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
/*
This is a main file for the TestApp which is used for integration testing.
This code supposed to be compiled with iPhone SDK and Xcode 3.1 Developer Tools for Mac OS X v10.5
*/
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int int_compar(const void *a, const void *b) {
	return *(int *)a - *(int *)b;
}

int test_qsort() {
	int arr[] = { 1, -1, 2, 1024, 4 };
	qsort(arr, sizeof(arr) / sizeof(int), sizeof(int), *int_compar);
	int expected_arr[] = { -1, 1, 2, 4, 1024 };
	return memcmp(arr, expected_arr, sizeof(expected_arr));
}

int main(int argc, char *argv[]) {
	int tests_run = 0;
	int tests_passed = 0;
	printf("test_qsort: ");
	tests_run++;
	if (test_qsort() == 0) {
		printf("OK\n");
		tests_passed++;
	} else {
		printf("FAIL\n");
	}
	printf("Passed %d out of %d tests\n", tests_passed, tests_run);
    return tests_run == tests_passed ? 0 : 1;
}
