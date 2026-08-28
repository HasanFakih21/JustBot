<div align="center">
<img width="283" height="242" alt="justbot_logo" src="https://github.com/user-attachments/assets/8b7be8b4-403a-4839-aeb5-d542a0c945d7" />
<h1>JustBot Chess Engine</h1>
  
[![License: AGPL v3.0](https://img.shields.io/github/license/HasanFakih21/JustBot?style=flat-square&color=orange)](https://www.gnu.org/licenses/agpl-3.0)
[![GitHub Release](https://img.shields.io/github/v/release/HasanFakih21/JustBot?include_prereleases&style=flat-square&color=green)](https://github.com/HasanFakih21/JustBot/releases)

</div>

<div align="center">JustBot is a competitive chess engine written in Rust, it's my attempt at learning the language while simultaneously working on something interesting to me; it has been a lot of fun and a huge learning experience and I hope to be able to continue working on it. This project has only been possible thanks to the wonderful open-source community. JustBot contains no LLM generated code.</div>

## Releases
|        Version             |       CCRL 40/15         |        CCRL Blitz      |   COPE Bullet  |   COPE Rapid  |
|         :---:              |         :---:            |           :---:        |     :---:      |     :---:     |
| [JustBot v0.4.0][v0.4.0]   |          3500*           |           3600*        |                |               |
| [JustBot v0.3.0][v0.3.0]   |          3400*           |           3400*        |   3426 [#37]   |   3467 [#40]  |
| [JustBot v0.2.0][v0.2.0]   |          3124 [#226]     |           3000*        |                |               |
| [JustBot v0.1.0][v0.1.0]   |          2400*           |           2400*        |                |               |

[v0.1.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.1.0
[v0.2.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.2.0
[v0.3.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.3.0
[v0.4.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.4.0

> [!NOTE]
> *Elo is only an estimate

You can find precompiled binaries for Linux, Windows and macOS [here](https://github.com/HasanFakih21/JustBot/releases)
- `avx512`: The fastest build, only compatible with newer CPUs
- `avx2`: Usable on most modern CPUs
- `generic`: The slowest build, should run on any x86-64 CPU.

## Building the project
To build the project, you need a working installation of Rust and Cargo, once the repository is cloned, you can run:
```bash
cargo build --release
# ./target/release/justbot
```
Otherwise, if you have GNU make installed you can just run:
```bash
make
# ./justbot
```


## Features
### Search
- Alpha-Beta Search
- Quiescence Search
    - Check Evasions
- Principal Variation Search
- Time management
    - Hard/Soft Bounds
    - Node Scaling
- SEE Pruning
- Iterative Deepening
- Null Move Pruning
- Reverse Futility Pruning
- Aspiration Windows
- Late Move Reductions
- Late Move Pruning
- Futility Pruning
- Clustered Transposition Table
- Improving Modifier
- Singular Extensions
    - Negative Extensions
    - Double Extensions
- History Pruning
- Razoring

### Evaluation
- NNUE
    - Standard 768 Inputs
    - Dual Perspective
    - 768 HL
    - 8 Output Buckets
    - Horizontally Mirrored
    - 3 Input Buckets
    - Fused Updates
    - Finny Tables
    - Lazy Updates

### Move Ordering
- Noisy History
- Quiet History
- 1 and 2 Ply Continuation Histories

### Supported UCI Options
| Name             |    Default   |       Max     |                Description                      |
| :---:            |     :---:    |      :---:    |                   :---:                         |
| Hash             |      16      |     1048576   | Sets the size of the transposition table in MB  |
| Clear Hash       |      ---     |       ---     | Clears all entries in the transposition table   |
| Threads          |       1      |       512     | Sets the number of threads to use during search |
| UCI_Chess960     |     false    |       ---     | Enables Chess960 support                        |

## Acknowledgments
- [Chess Programming Wiki](https://www.chessprogramming.org/Main_Page)
- [Maksim Korzh](https://www.youtube.com/watch?v=QUNP-UjujBM&list=PLmN0neTso3Jxh8ZIylk74JpwfiWNI76Cs) for helpful introductory videos, and where my magic numbers are from
- The very helpful members of the [Stockfish Discord Server](https://discord.com/invite/GWDRS3kU6R)
- [OpenBench](https://github.com/andygrant/openbench) as the testing framework, and for data generation
- [Bullet](https://github.com/jw1912/bullet) for NNUE training
- [Pawnocchio](https://github.com/JonathanHallstrom/pawnocchio) for converting PGNs to Viriformat

Additionally, the following engines have been huge sources of ideas and inspiration:
- [Reckless](https://github.com/codedeliveryservice/Reckless)
- [Stockfish](https://github.com/official-stockfish/stockfish)
- [Stormphrax](https://github.com/Ciekce/Stormphrax)
- [Viridithas](https://github.com/cosmobobak/viridithas)
- [Hobbes](https://github.com/kelseyde/hobbes-chess-engine)
- [Potential](https://github.com/ProgramciDusunur/Potential)
- [Pawnocchio](https://github.com/JonathanHallstrom/pawnocchio)
- [Icarus](https://github.com/Sp00ph/icarus)