<div align="center">
<img width="283" height="242" alt="justbot_logo" src="https://github.com/user-attachments/assets/8b7be8b4-403a-4839-aeb5-d542a0c945d7" />
<h1>JustBot Chess Engine</h1>
  
[![License: GPL-3.0](https://img.shields.io/github/license/HasanFakih21/JustBot?style=flat-square&color=blue)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![GitHub Release](https://img.shields.io/github/v/release/HasanFakih21/JustBot?include_prereleases&style=flat-square&color=green)](https://github.com/HasanFakih21/JustBot/releases)

</div>

<div align="center">JustBot is written without the use of any agentic or LLM assisted coding.</div>

## Releases
|        Version             |       CCRL 40/15         |        CCRL Blitz      |
|         :---:              |         :---:            |           :---:        |
| [JustBot v0.2.0][v0.2.0]   |          3124            |           3000*        |
| [JustBot v0.1.0][v0.1.0]   |          2400*           |           2400*        |

[v0.1.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.1.0
[v0.2.0]: https://github.com/HasanFakih21/JustBot/releases/tag/v0.2.0

> [!NOTE]
> *Elo is only an estimate

You can find precompiled binaries for Linux and Windows [here](https://github.com/HasanFakih21/JustBot/releases)

## Building the project
To build the project, you need a working installation of Rust and Cargo, once the repository is cloned, you can run for a general build:

```bash
cargo build --release
```

For a targeted  build you can run:

```bash
cargo rustc --release --bin justbot -- -C target-cpu=native
```

The binary should be located within `./target/release/`

## Features
- Alpha-Beta search
- Basic UCI compatibility
- Clustered Transposition Table
- (768 -> 128)x2 -> 1 NNUE
- Quiescence Search
- Iterative Deepening
- Time management
- Principal Variation Search
- Null Move Pruning
- MVV-LVA
- Aspiration Windows
- Reverse Futility Pruning
- Late Move Reductions
- Noisy and Quiet History
- Late Move Pruning
- Futility Pruning
- Check Extensions
- SEE Pruning in Qsearch

### Supported UCI Options
| Name        |    Default   |                Description                     |
| :---:       |     :---:    |                   :---:                        |
| Hash        |      16      | Sets the size of the transposition table in MB |
| Clear Hash  |      ---     | Clears all entries in the transposition table  |

## Acknowledgments
- [Chess Programming Wiki](https://www.chessprogramming.org/Main_Page)
- [Maksim Korzh](https://www.youtube.com/watch?v=QUNP-UjujBM&list=PLmN0neTso3Jxh8ZIylk74JpwfiWNI76Cs) for helpful introductory videos, and where my magic numbers are from
- [Reckless](https://github.com/codedeliveryservice/Reckless) and [Stockfish](https://github.com/official-stockfish/stockfish)
- The very helpful members of the [Stockfish Discord Server](https://discord.com/invite/GWDRS3kU6R)
- [OpenBench](https://github.com/andygrant/openbench) as the testing framework, and for data generation
- [Bullet](https://github.com/jw1912/bullet) for NNUE training