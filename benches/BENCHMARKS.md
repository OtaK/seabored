# Benchmarks

## Table of Contents

- [Benchmark Results](#benchmark-results)
    - [mimi_content_multipart_3](#mimi_content_multipart_3)
    - [log](#log)
    - [mesh](#mesh)
    - [minecraft_savedata](#minecraft_savedata)
    - [mk48](#mk48)

## Benchmark Results

Ran on `Linux 7.0.12-x86_64 #1 SMP PREEMPT_DYNAMIC x86_64 AMD Ryzen 9 5950X 16-Core Processor AuthenticAMD GNU/Linux`

### mimi_content_multipart_3

|                 | `seabored`                | `cbor4ii`                        | `ciborium`                        |
|:----------------|:--------------------------|:---------------------------------|:--------------------------------- |
| **`value-de`**  | `814.29 ns` (✅ **1.00x**) | `1.19 us` (❌ *1.46x slower*)     | `4.61 us` (❌ *5.66x slower*)      |
| **`value-ser`** | `355.45 ns` (✅ **1.00x**) | `530.59 ns` (❌ *1.49x slower*)   | `598.69 ns` (❌ *1.68x slower*)    |
| **`serde-de`**  | `881.22 ns` (✅ **1.00x**) | `777.91 ns` (✅ **1.13x faster**) | `3.24 us` (❌ *3.68x slower*)      |
| **`serde-ser`** | `209.48 ns` (✅ **1.00x**) | `198.33 ns` (✅ **1.06x faster**) | `424.58 ns` (❌ *2.03x slower*)    |

### log

|                 | `seabored`                | `cbor4ii`                        | `ciborium`                       |
|:----------------|:--------------------------|:---------------------------------|:-------------------------------- |
| **`value-de`**  | `3.91 ms` (✅ **1.00x**)   | `6.34 ms` (❌ *1.62x slower*)     | `15.23 ms` (❌ *3.89x slower*)    |
| **`value-ser`** | `1.32 ms` (✅ **1.00x**)   | `1.78 ms` (❌ *1.35x slower*)     | `2.32 ms` (❌ *1.76x slower*)     |
| **`serde-de`**  | `1.48 ms` (✅ **1.00x**)   | `2.67 ms` (❌ *1.80x slower*)     | `7.18 ms` (❌ *4.83x slower*)     |
| **`serde-ser`** | `450.44 us` (✅ **1.00x**) | `589.70 us` (❌ *1.31x slower*)   | `1.50 ms` (❌ *3.34x slower*)     |

### mesh

|                 | `seabored`               | `cbor4ii`                       | `ciborium`                        |
|:----------------|:-------------------------|:--------------------------------|:--------------------------------- |
| **`value-de`**  | `80.22 ms` (✅ **1.00x**) | `99.45 ms` (❌ *1.24x slower*)   | `254.98 ms` (❌ *3.18x slower*)    |
| **`value-ser`** | `23.63 ms` (✅ **1.00x**) | `32.93 ms` (❌ *1.39x slower*)   | `45.63 ms` (❌ *1.93x slower*)     |
| **`serde-de`**  | `15.44 ms` (✅ **1.00x**) | `37.85 ms` (❌ *2.45x slower*)   | `87.02 ms` (❌ *5.63x slower*)     |
| **`serde-ser`** | `10.34 ms` (✅ **1.00x**) | `10.83 ms` (✅ **1.05x slower**) | `28.51 ms` (❌ *2.76x slower*)     |

### minecraft_savedata

|                 | `seabored`                | `cbor4ii`                        | `ciborium`                       |
|:----------------|:--------------------------|:---------------------------------|:-------------------------------- |
| **`value-de`**  | `3.69 ms` (✅ **1.00x**)   | `5.61 ms` (❌ *1.52x slower*)     | `13.84 ms` (❌ *3.75x slower*)    |
| **`value-ser`** | `1.15 ms` (✅ **1.00x**)   | `1.53 ms` (❌ *1.33x slower*)     | `2.11 ms` (❌ *1.83x slower*)     |
| **`serde-de`**  | `1.51 ms` (✅ **1.00x**)   | `2.44 ms` (❌ *1.62x slower*)     | `6.49 ms` (❌ *4.30x slower*)     |
| **`serde-ser`** | `456.98 us` (✅ **1.00x**) | `564.43 us` (❌ *1.24x slower*)   | `1.56 ms` (❌ *3.42x slower*)     |

### mk48

|                 | `seabored`               | `cbor4ii`                       | `ciborium`                       |
|:----------------|:-------------------------|:--------------------------------|:-------------------------------- |
| **`value-de`**  | `25.00 ms` (✅ **1.00x**) | `35.83 ms` (❌ *1.43x slower*)   | `94.43 ms` (❌ *3.78x slower*)    |
| **`value-ser`** | `8.08 ms` (✅ **1.00x**)  | `10.28 ms` (❌ *1.27x slower*)   | `14.97 ms` (❌ *1.85x slower*)    |
| **`serde-de`**  | `7.67 ms` (✅ **1.00x**)  | `11.99 ms` (❌ *1.56x slower*)   | `35.90 ms` (❌ *4.68x slower*)    |
| **`serde-ser`** | `2.12 ms` (✅ **1.00x**)  | `1.86 ms` (✅ **1.14x faster**)  | `10.21 ms` (❌ *4.82x slower*)    |

---
Made with [criterion-table](https://github.com/nu11ptr/criterion-table)
