# rstat

Performance golfing an Ubuntu `ufetch` clone in Rust (for fun).

## Perf

Very fast!

```
$ sudo perf stat -e cycles:u,cycles:k,instructions:u,instructions:k -r 500 ./rstat
<SNIP>
 Performance counter stats for './rstat' (500 runs):

            20,365      cycles:u                                                                ( +-  0.62% )
           137,662      cycles:k                                                                ( +-  1.60% )
            13,644      instructions:u                                                          ( +-  0.00% )
           188,591      instructions:k                                                          ( +-  0.55% )

       0.000244756 +- 0.000003830 seconds time elapsed  ( +-  1.56% )

$ sudo perf trace -s ./rstat

     ---(_)             os:       Ubuntu 26.04 LTS (Resolute Raccoon) Resolute
 _/  ---  \             kernel:   Linux 7.0.0-28-generic
(_) |   |               uptime:   5h 37m
  \  --- _/             disk:     34812/475863
     ---(_)             mem:      28769.39/30640.79     


 Summary of events:

 rstat (139171), 20 events, 80.0%

   syscall            calls  errors  total       min       avg       max       stddev
                                     (msec)    (msec)    (msec)    (msec)        (%)
   --------------- --------  ------ -------- --------- --------- ---------     ------
   write                  1      0     0.009     0.009     0.009     0.009      0.00%
   open                   1      0     0.006     0.006     0.006     0.006      0.00%
   sysinfo                1      0     0.005     0.005     0.005     0.005      0.00%
   statfs                 1      0     0.004     0.004     0.004     0.004      0.00%
   read                   1      0     0.001     0.001     0.001     0.001      0.00%
   close                  1      0     0.001     0.001     0.001     0.001      0.00%
   arch_prctl             1      0     0.001     0.001     0.001     0.001      0.00%
   uname                  1      0     0.001     0.001     0.001     0.001      0.00%
   set_tid_address        1      0     0.001     0.001     0.001     0.001      0.00%
   execve                 1      0     0.000     0.000     0.000     0.000      0.00%
```

Since `ufetch` has more features and is a shell script, it is obviously much slower by nature. 
But for fun, here's just how much:

```
$ sudo perf stat -e cycles:u,cycles:k,instructions:u,instructions:k -r 500 ./ufetch
<SNIP>
 Performance counter stats for './ufetch' (500 runs):

        66,430,332      cycles:u                                                                ( +-  0.09% )
        80,875,566      cycles:k                                                                ( +-  0.15% )
       152,179,777      instructions:u                                                          ( +-  0.00% )
       107,271,857      instructions:k                                                          ( +-  0.04% )

       0.025543752 +- 0.000046731 seconds time elapsed  ( +-  0.18% )
```