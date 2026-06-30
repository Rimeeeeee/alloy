# Engine API JSON vs SSZ Codec Benchmark

Source data:

- Execution blocks and block access lists: `https://ethereum.reth.rs/rpc`
- Sample: 20 consecutive Ethereum mainnet blocks
- Average transactions per block: 364.6
- Average block access list size: 103.7 KB
- Real blob sidecars: 8 blobs from execution block `0x1840279`, beacon slot `14664638`
- Blob sidecar source: public Ethereum beacon API. The public execution RPC exposes versioned hashes but not `engine_getBlobsV1` or `engine_getBlobsV2`.

Run command:

```sh
cargo bench --package alloy-rpc-types-engine --bench codec --features ssz
```

## HackMD

Paste this Markdown directly into HackMD. The tables render without extra setup.

For the SVG, first push the branch and use a public raw GitHub URL:

```md
![Engine API codec benchmark infographic](https://raw.githubusercontent.com/<org>/<repo>/<branch>/crates/rpc-types-engine/benches/codec-results.svg)
```

or:

```html
<img src="https://raw.githubusercontent.com/<org>/<repo>/<branch>/crates/rpc-types-engine/benches/codec-results.svg" width="100%">
```

If HackMD refuses raw SVG, upload the SVG as a HackMD attachment or convert it to PNG and embed the uploaded PNG URL.

## Summary

This is codec-only. Fixture parsing, RPC fetching, block decoding, BAL extraction, and payload construction are outside measured loops. The benchmark measures:

- `serde_json::to_vec`
- `ssz::Encode::as_ssz_bytes`
- `serde_json::from_slice`
- `ssz::Decode::from_ssz_bytes`

SSZ is consistently smaller on the wire. For execution payload-like structs, JSON is about 2x the SSZ byte size. Encode/decode speedups vary by type: payload rows are often 9-15x faster to encode and 4-6x faster to decode with SSZ, while very large bodies/blob responses see lower encode speedups but still strong decode wins.

## newPayload - serde

| Version | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| V1 (paris) | avg | 316.0 KB -> 158.5 KB | 372.6 us | 29.2 us | 12.8x | 192.5 us | 44.1 us | 4.4x |
| V1 (paris) | max | 610.6 KB -> 305.7 KB | 776.4 us | 53.7 us | 14.5x | 289.8 us | 43.0 us | 6.7x |
| V2 (shanghai) | avg | 318.0 KB -> 159.2 KB | 378.8 us | 32.3 us | 11.7x | 197.4 us | 44.1 us | 4.5x |
| V2 (shanghai) | max | 612.6 KB -> 306.4 KB | 774.8 us | 68.1 us | 11.4x | 297.4 us | 43.7 us | 6.8x |
| V3 (cancun-osaka) | avg | 318.1 KB -> 159.2 KB | 378.5 us | 30.8 us | 12.3x | 198.3 us | 44.6 us | 4.4x |
| V3 (cancun-osaka) | max | 612.7 KB -> 306.4 KB | 781.9 us | 46.1 us | 17.0x | 303.4 us | 44.7 us | 6.8x |
| V4 (amsterdam) | avg | 525.4 KB -> 262.9 KB | 632.5 us | 40.9 us | 15.5x | 285.0 us | 50.8 us | 5.6x |
| V4 (amsterdam) | max | 974.9 KB -> 488.8 KB | 1.22 ms | 62.3 us | 19.7x | 555.0 us | 126.2 us | 4.4x |

## getPayload - serde

Envelope blob data uses correctly sized zero-filled blobs based on each block's blob gas usage. Average blob count: 3.0 per block.

| Version | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| V2 (paris form) | avg | 316.1 KB -> 158.5 KB | 374.3 us | 33.9 us | 11.1x | 198.3 us | 44.4 us | 4.5x |
| V2 (paris form) | max | 610.7 KB -> 305.7 KB | 795.6 us | 57.1 us | 13.9x | 298.2 us | 43.6 us | 6.8x |
| V3 (cancun) | avg | 1.11 MB -> 552.8 KB | 1.49 ms | 225.5 us | 6.6x | 654.4 us | 164.6 us | 4.0x |
| V3 (cancun) | max | 2.97 MB -> 1.49 MB | 4.14 ms | 1.18 ms | 3.5x | 1.95 ms | 671.8 us | 2.9x |
| V4 (prague) | avg | 1.11 MB -> 552.8 KB | 1.50 ms | 293.9 us | 5.1x | 655.2 us | 161.6 us | 4.1x |
| V4 (prague) | max | 2.97 MB -> 1.49 MB | 4.12 ms | 1.15 ms | 3.6x | 1.90 ms | 657.4 us | 2.9x |
| V5 (osaka) | avg | 1.14 MB -> 571.1 KB | 1.57 ms | 242.9 us | 6.5x | 708.4 us | 178.3 us | 4.0x |
| V5 (osaka) | max | 3.09 MB -> 1.54 MB | 4.26 ms | 1.18 ms | 3.6x | 2.03 ms | 726.8 us | 2.8x |
| V6 (amsterdam) | avg | 1.35 MB -> 674.7 KB | 1.92 ms | 439.7 us | 4.4x | 835.2 us | 178.2 us | 4.7x |
| V6 (amsterdam) | max | 3.44 MB -> 1.72 MB | 5.06 ms | 1.20 ms | 4.2x | 2.34 ms | 725.9 us | 3.2x |

## REST newPayload fork containers - serde

| Version | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Paris | avg | 316.0 KB -> 158.5 KB | 383.6 us | 30.0 us | 12.8x | 189.5 us | 43.7 us | 4.3x |
| Paris | max | 610.6 KB -> 305.7 KB | 772.2 us | 53.1 us | 14.5x | 291.0 us | 43.2 us | 6.7x |
| Shanghai | avg | 318.0 KB -> 159.2 KB | 379.8 us | 38.3 us | 9.9x | 199.0 us | 44.2 us | 4.5x |
| Shanghai | max | 612.6 KB -> 306.4 KB | 788.5 us | 50.7 us | 15.6x | 299.9 us | 43.4 us | 6.9x |
| Cancun | avg | 318.2 KB -> 159.2 KB | 385.5 us | 42.6 us | 9.1x | 200.6 us | 43.9 us | 4.6x |
| Cancun | max | 612.8 KB -> 306.5 KB | 784.8 us | 78.8 us | 10.0x | 304.0 us | 44.4 us | 6.8x |
| Prague | avg | 318.2 KB -> 159.3 KB | 387.4 us | 41.3 us | 9.4x | 199.1 us | 43.9 us | 4.5x |
| Prague | max | 612.8 KB -> 306.5 KB | 784.3 us | 56.5 us | 13.9x | 299.7 us | 44.1 us | 6.8x |
| Osaka | avg | 318.2 KB -> 159.3 KB | 374.0 us | 38.2 us | 9.8x | 198.5 us | 43.8 us | 4.5x |
| Osaka | max | 612.8 KB -> 306.5 KB | 780.4 us | 59.5 us | 13.1x | 300.1 us | 43.6 us | 6.9x |
| Amsterdam | avg | 525.6 KB -> 262.9 KB | 637.8 us | 53.3 us | 12.0x | 282.1 us | 49.7 us | 5.7x |
| Amsterdam | max | 975.1 KB -> 488.9 KB | 1.22 ms | 85.9 us | 14.2x | 553.4 us | 126.3 us | 4.4x |

## REST builtPayload fork containers - serde

| Version | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Paris | avg | 316.1 KB -> 158.5 KB | 369.8 us | 34.0 us | 10.9x | 189.0 us | 43.6 us | 4.3x |
| Paris | max | 610.7 KB -> 305.7 KB | 781.2 us | 56.6 us | 13.8x | 292.1 us | 42.9 us | 6.8x |
| Shanghai | avg | 318.1 KB -> 159.2 KB | 381.8 us | 41.6 us | 9.2x | 197.0 us | 43.9 us | 4.5x |
| Shanghai | max | 612.7 KB -> 306.4 KB | 785.3 us | 78.0 us | 10.1x | 298.5 us | 44.3 us | 6.7x |
| Cancun | avg | 638.1 KB -> 319.1 KB | 847.2 us | 65.8 us | 12.9x | 334.0 us | 51.5 us | 6.5x |
| Cancun | max | 638.1 KB -> 319.1 KB | 830.1 us | 64.8 us | 12.8x | 331.1 us | 46.5 us | 7.1x |
| Prague | avg | 1.11 MB -> 552.8 KB | 1.47 ms | 224.2 us | 6.6x | 647.9 us | 158.1 us | 4.1x |
| Prague | max | 2.97 MB -> 1.49 MB | 4.11 ms | 1.13 ms | 3.6x | 1.88 ms | 639.4 us | 2.9x |
| Osaka | avg | 1.14 MB -> 571.1 KB | 1.59 ms | 259.9 us | 6.1x | 715.6 us | 179.5 us | 4.0x |
| Osaka | max | 3.09 MB -> 1.54 MB | 4.27 ms | 1.20 ms | 3.6x | 2.04 ms | 700.8 us | 2.9x |
| Amsterdam | avg | 1.35 MB -> 674.7 KB | 1.81 ms | 320.6 us | 5.6x | 772.9 us | 174.3 us | 4.4x |
| Amsterdam | max | 3.44 MB -> 1.72 MB | 4.64 ms | 1.22 ms | 3.8x | 2.17 ms | 665.2 us | 3.3x |

## REST payload body fork containers - serde

| Version | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Paris | avg | 314.8 KB -> 158.0 KB | 347.4 us | 27.2 us | 12.8x | 185.6 us | 41.9 us | 4.4x |
| Paris | max | 609.5 KB -> 305.2 KB | 627.2 us | 44.2 us | 14.2x | 287.2 us | 41.5 us | 6.9x |
| Shanghai-Cancun-Prague-Osaka | avg | 316.8 KB -> 158.7 KB | 344.1 us | 32.7 us | 10.5x | 197.1 us | 42.8 us | 4.6x |
| Shanghai-Cancun-Prague-Osaka | max | 611.5 KB -> 305.9 KB | 624.9 us | 59.4 us | 10.5x | 294.3 us | 42.8 us | 6.9x |
| Amsterdam | avg | 524.2 KB -> 262.3 KB | 608.2 us | 45.7 us | 13.3x | 278.5 us | 48.3 us | 5.8x |
| Amsterdam | max | 973.7 KB -> 488.3 KB | 1.23 ms | 83.7 us | 14.7x | 546.8 us | 122.8 us | 4.5x |

## REST payload bodies requests/responses - serde

| Type | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| BodiesByHashRequest(20) | avg | 1.4 KB -> 0.6 KB | 1.8 us | 234.6 ns | 7.6x | 2.0 us | 170.0 ns | 12.0x |
| BodiesByHashRequest(20) | max | 1.4 KB -> 0.6 KB | 1.8 us | 236.0 ns | 7.5x | 2.1 us | 180.7 ns | 11.4x |
| BodiesResponse Paris(20) | avg | 6.30 MB -> 3.16 MB | 9.88 ms | 3.72 ms | 2.7x | 3.82 ms | 966.2 us | 4.0x |
| BodiesResponse Paris(20) | max | 6.30 MB -> 3.16 MB | 9.85 ms | 4.02 ms | 2.4x | 3.82 ms | 959.2 us | 4.0x |
| BodiesResponse Shanghai-Cancun-Prague-Osaka(20) | avg | 6.34 MB -> 3.17 MB | 9.97 ms | 4.29 ms | 2.3x | 3.99 ms | 976.8 us | 4.1x |
| BodiesResponse Shanghai-Cancun-Prague-Osaka(20) | max | 6.34 MB -> 3.17 MB | 9.95 ms | 3.93 ms | 2.5x | 4.00 ms | 982.3 us | 4.1x |
| BodiesResponse Amsterdam(20) | avg | 10.48 MB -> 5.25 MB | 16.83 ms | 5.96 ms | 2.8x | 5.79 ms | 1.10 ms | 5.3x |
| BodiesResponse Amsterdam(20) | max | 10.48 MB -> 5.25 MB | 16.79 ms | 5.87 ms | 2.9x | 5.73 ms | 1.10 ms | 5.2x |

## REST blob requests/responses - serde

| Type | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| BlobsV1Request / V2 / V3 real hashes | avg | 0.6 KB -> 0.3 KB | 835.1 ns | 215.1 ns | 3.9x | 876.5 ns | 163.1 ns | 5.4x |
| BlobsV1Request / V2 / V3 real hashes | max | 0.6 KB -> 0.3 KB | 832.8 ns | 214.0 ns | 3.9x | 875.5 ns | 171.1 ns | 5.1x |
| BlobsV4Request | avg | 0.6 KB -> 0.3 KB | 908.5 ns | 218.2 ns | 4.2x | 1.0 us | 166.9 ns | 6.1x |
| BlobsV4Request | max | 0.6 KB -> 0.3 KB | 903.4 ns | 219.3 ns | 4.1x | 1.0 us | 176.7 ns | 5.8x |
| BlobsV1Response(8 real) | avg | 2.10 MB -> 1.05 MB | 3.11 ms | 966.0 us | 3.2x | 923.6 us | 80.5 us | 11.5x |
| BlobsV1Response(8 real) | max | 2.10 MB -> 1.05 MB | 3.13 ms | 965.5 us | 3.2x | 911.7 us | 79.8 us | 11.4x |
| BlobsV2Response(8 real blobs, proof-shaped) | avg | 2.20 MB -> 1.10 MB | 3.24 ms | 1.39 ms | 2.3x | 1.04 ms | 161.5 us | 6.5x |
| BlobsV2Response(8 real blobs, proof-shaped) | max | 2.20 MB -> 1.10 MB | 3.22 ms | 1.40 ms | 2.3x | 1.05 ms | 105.6 us | 10.0x |
| BlobsV3Response(8 real blobs, proof-shaped) | avg | 2.20 MB -> 1.10 MB | 3.26 ms | 1.53 ms | 2.1x | 1.04 ms | 157.4 us | 6.6x |
| BlobsV3Response(8 real blobs, proof-shaped) | max | 2.20 MB -> 1.10 MB | 3.29 ms | 1.56 ms | 2.1x | 1.05 ms | 102.3 us | 10.3x |
| BlobsV4Response(1 partial real cells, proof-shaped) | avg | 12.7 KB -> 6.3 KB | 12.5 us | 2.5 us | 4.9x | 6.8 us | 1.3 us | 5.1x |
| BlobsV4Response(1 partial real cells, proof-shaped) | max | 12.7 KB -> 6.3 KB | 12.4 us | 2.5 us | 4.9x | 6.8 us | 1.3 us | 5.2x |

## REST payload witness response - serde

| Type | Case | Wire JSON -> SSZ | Encode JSON | Encode SSZ | Encode speedup | Decode JSON | Decode SSZ | Decode speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| PayloadStatusWithWitness none/full | avg | 9.4 KB -> 3.1 KB | 14.5 us | 3.9 us | 3.7x | 42.5 us | 841.8 ns | 50.4x |
| PayloadStatusWithWitness none/full | max | 18.6 KB -> 6.2 KB | 35.9 us | 7.5 us | 4.8x | 85.1 us | 1.3 us | 63.6x |

## Notes

- `newPayload`, `getPayload`, REST payload containers, bodies, blob rows, and witness rows all benchmark JSON and SSZ encode/decode now.
- The blob V1 response uses real blob bytes and real EIP-4844 KZG proofs from beacon sidecars.
- The blob V2/V3/V4 rows use real blob bytes or real blob-derived cells, with proof-shaped placeholder cell-proof lists where the public data source does not expose Engine API cell proofs.
- End-to-end HTTP benchmarking still belongs in Reth; this Alloy benchmark isolates codec cost.
