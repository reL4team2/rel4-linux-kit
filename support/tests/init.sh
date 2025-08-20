echo Hello World!

# ./brk
# ./chdir
# ./clone
# ./close
# ./dup
# ./dup2
# ./execve
# ./exit
# ./fork
# ./fstat
# ./getcwd
# ./getdents
# ./getpid
# ./getppid
# ./gettimeofday
# ./mkdir_
# ./mmap
# ./mount
# ./munmap
# ./open
# ./openat
# ./pipe
# ./read
# ./sleep
# ./test_echo
# ./umount
# ./uname
# ./unlink
# ./wait
# ./waitpid
# ./write
# ./yield

./busybox ash run-static.sh
./busybox sh busybox_testcode.sh
./busybox sh lua_testcode.sh
./busybox sh iozone_testcode.sh
./libc-bench
./busybox sh lmbench_testcode.sh
echo Exec Done
