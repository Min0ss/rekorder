# rekorder

Enregistreur de clips pour Windows 10/11, en Rust. Gratuit, sans réseau ni télémétrie. Plan et jalons : `plan.md`.

Priorités, dans l'ordre : impact minimal sur les FPS > qualité > fonctionnalités.

## Règles
- `unsafe` uniquement dans `crates/rk-win`, avec `// SAFETY:` sur chaque bloc. Les autres crates ont `#![forbid(unsafe_code)]` (vérifié par `crates/rk-app/tests/unsafe_policy.rs`).
- Les images restent sur le GPU (textures D3D11) de la capture à l'encodeur. Pas de copie CPU.
- Canaux bornés : on perd des images plutôt que de laisser la mémoire grossir, et on compte les pertes.
- Pas de dépendance GPL ni réseau (`deny.toml`). Vérifier toute nouvelle crate sur crates.io avant de l'ajouter.
- Aucun chiffre de performance sans mesure (méthode : `plan.md` §5).

## Vérification (avant de clore un jalon)
```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo audit
```
