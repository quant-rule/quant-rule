# QuantRule - Architecture Characteristics

Eight characteristics matter. Each is tied to a numbered requirement and has a
level that can be measured. The rest are deliberately not optimised.

| Characteristic | Kata | Why it matters | Level (measurable) | Measured by |
|:---------------|:----:|:---------------|:------------------|:------------|
| Testability | R2, R3, R5 | every published number is a claim someone defends | every reading and event has a causality, a determinism, and a golden test; fast suite <= 5 min | `cargo test`, gate |
| Modularity | R4, R6, R7 | six services only pay off if boundaries hold | 6 services, 0 cycles, gateway imports 0 engine symbols | boundary test |
| Maintainability | R3, R7 | the rule changes weekly, the platform must not | a rule change touches <= 1 crate and <= 3 modules; no file over 1,000 lines | structural check |
| Deployability | R4, R6 | a reviewer must reproduce in one command | clean clone to healthy <= 10 min; 1 published port; desktop is 1 asset | `make up` |
| Fault tolerance | R4, R5 | a run must survive a crash without corrupting evidence | 0 succeeded runs with a missing artifact; resubmission gives identical bytes | crash test |
| Responsiveness | R4, R7 | marking is interactive, a year of candles is not | submit <= 200 ms; chart window <= 200 ms; 1 year, 8 timeframes <= 60 s on 8 cores | benchmark |
| Scalability | R4 | one machine must borrow a second | model stage 1 to 4 workers: identical digest, >= 2.5x throughput | scale test |
| Partitioning type | R1-R7 | capability boundaries keep a rule change inside one service | domain-driven: 6 services by capability, no layer split | service catalog |

**Not optimised:** elasticity (no burst traffic, one operator, batch work),
localization (one language, one user), and accessibility (one expert user;
keyboard support is kept, but it is not a driver).
