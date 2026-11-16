# MolibdenDOOM

**MolibdenDOOM** is a modern source port based on the original [id Software DOOM source code]([https://github.com](https://github.com/id-Software/Doom)), specifically tailored for **64-bit Linux** systems.

## About the Project

The primary goal of MolibdenDOOM is to revitalize the classic 1993 shooter for contemporary operating systems by replacing legacy dependencies with modern libraries such as **SDL3**, while significantly enhancing the gameplay experience with responsive, modern controls.

### Key Features
* **64-bit Linux Support:** Refactored and adapted to compile and run natively on modern x86_64 Linux distributions.
* **Modern Backend:** Legacy video, audio, and input subsystems replaced with **SDL3** and modern equivalents.
* **Enhanced Controls:** Updated input handling designed for comfortable, modern mouse-and-keyboard gameplay.

## Roadmap

* [x] Port original codebase to 64-bit Linux
* [x] Integrate SDL3 for windowing, rendering, and input
* [ ] **Next Major Milestone:** Rewrite and migrate the entire source port to **Rust** for improved memory safety, maintainability, and performance.

## Building from Source

### Prerequisites
* A 64-bit Linux environment
* CMake (3.10 or higher)
* C/C++ compiler (GCC or Clang)
* SDL3 development libraries

### Compilation
```bash
git clone https://github.com/molibdendeveloper-star/MolibdenDOOM
cd MolibdenDOOM
mkdir build && cd build
cmake ..
cmake --build .
```

## Acknowledgments & Transparency

* **Original Codebase:** Based on the official open-source release by [id Software]([https://github.com](https://github.com/id-Software/Doom)).
* **AI Assistance:** AI assistants were utilized during the development of this project for refactoring, debugging, and modernization tasks.

## License

This project is licensed under the **GNU General Public License v2.0** — see the [LICENSE.TXT](LICENSE.TXT) file for details, in accordance with the original id Software release.
