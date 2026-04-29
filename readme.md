
## Initial result

```log
test faust_10_sine_oscillators  ... bench:      18,507.43 ns/iter (+/- 1,049.02) = 221 MB/s
test mimium_10_sine_oscillators ... bench:     325,204.56 ns/iter (+/- 16,998.27) = 50 MB/s
```

## Frame-wide execution in mimium code


```log
test faust_10_sine_oscillators  ... bench:      18,617.05 ns/iter (+/- 747.16) = 220 MB/s
test mimium_10_sine_oscillators ... bench:     303,513.67 ns/iter (+/- 42,895.46) = 53 MB/s
```

## Forcing inlining

```log
test faust_10_sine_oscillators  ... bench:      18,673.48 ns/iter (+/- 781.63) = 219 MB/s
test mimium_10_sine_oscillators ... bench:     225,061.12 ns/iter (+/- 37,800.61) = 72 MB/s
```