This is a halo2 circuit that verifies the Linear Congruential Generator(https://en.wikipedia.org/wiki/Linear_congruential_generator).

Developed in Ethcon 2023 Seoul for a zkDraw project(https://github.com/jae-cuz/zk-draw)


# test

To test,
```
cargo test -- --nocapture test_rand
```


To print layout,
```
cargo test --all-features -- --nocapture print_test_rand
```
