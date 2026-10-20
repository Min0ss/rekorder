# rekorder — Plan

*Rédigé le 2026-10-20. Phase 1 validée le 2026-10-20.*

## Décisions validées (2026-10-20)

- **Architecture A** : Media Foundation natif, sans FFmpeg.
- **Codec par défaut : H.264**, débit variable piloté par qualité (CQP/VBR selon ce que le MFT expose). Raisons :
  - C'est le seul encodeur matériel présent sur quasiment toute carte encore en service (NVIDIA, AMD, Intel) **[S]**, ce qui sert l'objectif « n'importe quelle carte ».
  - Les clips se lisent partout sans réencodage : Discord, navigateurs, monteurs, téléphones.
  - Le MFT H.264 logiciel de Windows sert de repli.
  - **HEVC et AV1 sont proposés en option** quand le GPU les expose (détection à l'exécution). Ils offrent une meilleure qualité à débit égal et un buffer RAM plus léger, mais la lecture est moins universelle.
- **Replay buffer : RAM uniquement**, avec une limite stricte en Mo. Le débordement sur disque est reporté.
- **Cible matérielle** : aucune exigence au-delà d'un GPU D3D11 ; repli logiciel sinon. La machine de dev (RTX 4070 Super 12 Go, 64 Go de RAM) est le haut de gamme ; il faudra aussi valider sur une carte modeste ou un iGPU avant la 1.0.

Légende : **[V]** vérifié dans une source officielle ou sur crates.io aujourd'hui · **[NV]** non vérifié · **[S]** supposition.

---

## 0. État des lieux

- Dossier vide, pas de dépôt git. **[V]**
- Toolchain : rustc 1.94.0, cargo 1.94.0, git 2.52. **[V]**
- Machine de dev : Windows 11 Pro 26200. **[V]** GPU inconnu, à relever avant le jalon 2.

## 1. Crates et API vérifiées

| Brique | Candidat | Version / licence | Statut |
|---|---|---|---|
| Bindings Win32/WinRT | `windows` (Microsoft) | 0.62.2, MIT OR Apache-2.0 | [V] |
| Capture | `windows-capture` | 2.0.1, MIT. Utilise WGC et propose aussi DXGI Desktop Duplication. Inclut un `VideoEncoder` « matériel » dont l'API sous-jacente n'est pas documentée dans le README | [V] (version/licence) / [NV] (encodeur interne) |
| Audio | `wasapi` (HEnquist) | 0.26.0, MIT. Énumération, capture loopback, notifications de périphérique. Loopback par processus : seulement via `ActivateAudioInterfaceAsync` appelé à la main | [V] |
| Raccourcis globaux | `global-hotkey` (tauri-apps) | 0.9.0, Apache-2.0 OR MIT | [V] |
| Icône de notification | `tray-icon` (tauri-apps) | 0.26.1, MIT OR Apache-2.0 | [V] |
| FFmpeg (option B) | `ffmpeg-next` | 9.0.0, WTFPL (wrapper). FFmpeg lui-même est en LGPL si compilé **sans** `--enable-gpl`/`--enable-nonfree`, avec liaison dynamique (DLL) et publication des sources | [V] |
| Muxer MKV | `oxideav-mkv` | 0.0.x, maturité et licence non vérifiées. `matroska` et `matroska-demuxer` ne font que la lecture | [NV] |

API Windows :
- **Encodeurs matériels Media Foundation** : MFT asynchrones obligatoires, avec changement de format dynamique. **[V]** Énumération via `MFTEnumEx` + `MFT_ENUM_FLAG_HARDWARE` et entrée D3D11 via `IMFDXGIDeviceManager` : **[S]** (connu, pas relu aujourd'hui).
- **Bordure jaune WGC** : `IsBorderRequired` existe depuis la build 20348. Pour la désactiver, il faut le consentement via `GraphicsCaptureAccess.RequestAccessAsync(Borderless)` et la capability `graphicsCaptureWithoutBorder` dans le manifeste d'une app **packagée**. **[V]** Le comportement d'une app Win32 non packagée reste **[NV]** : c'est un risque.

## 2. Architectures comparées

Toutes partent d'une capture GPU (texture D3D11) **sans copie vers la RAM** : c'est la condition n°1 pour limiter l'impact sur les FPS.

### A. WGC → conversion GPU → MFT matériel Media Foundation → muxer MKV maison *(recommandée)*
- Capture : Windows.Graphics.Capture (fenêtre, écran) ; DXGI Desktop Duplication en repli.
- Conversion BGRA→NV12 sur GPU (`ID3D11VideoProcessor`), puis encodeur MFT H.264/HEVC (NVENC/AMF/QSV exposés par les pilotes). Repli logiciel : MFT H.264 de Windows.
- Audio : WASAPI loopback + micro, encodage AAC par le MFT Windows.
- Conteneur : petit muxer Matroska écrit en Rust (EBML en écriture, environ 1 à 2 kLOC **[S]**). Remux MP4 sans réencodage via `IMFSinkWriter` en entrée compressée **[S, à vérifier]**.
- ✅ Aucune dépendance native à distribuer, binaire de quelques Mo **[S]**, aucune question de licence FFmpeg, codecs fournis par l'OS ou les pilotes.
- ❌ Le contrôle du débit dépend de ce que chaque MFT expose (`ICodecAPI`) et varie selon le fournisseur **[S]**. AV1 MFT disponible selon GPU et pilote **[NV]**. Beaucoup de code COM `unsafe`.

### B. WGC → FFmpeg (LGPL, DLL) avec hwframes D3D11 → nvenc/amf/qsv → muxer FFmpeg
- ✅ Encodeurs et muxers éprouvés (MKV, MP4, fragmentation), réglages fins par fournisseur, AV1.
- ❌ Plusieurs dizaines de Mo de DLL **[S]**, obligations LGPL (sources, mention, build reproductible), build Windows de FFmpeg pénible, plus grande surface d'attaque, et une partie de la chaîne en C.

### C. SDK fournisseurs en direct (NVENC SDK, AMF, oneVPL)
- ✅ Contrôle et latence maximaux, comme dans ShadowPlay.
- ❌ Trois implémentations à maintenir, licences des en-têtes SDK à étudier **[NV]**, trois matériels de test nécessaires.

**Recommandation : A.** C'est la plus légère et la plus simple en licences, et elle reste entièrement GPU. Point de contrôle au jalon 2 : si la qualité à débit égal ou le réglage du débit des MFT sont mesurés comme insuffisants face à OBS (mêmes réglages), on bascule l'encodage sur B en gardant capture, audio et buffer. L'interface `Encoder` est conçue pour rendre ce changement possible.

## 3. Architecture logicielle (A)

```
crates/
  rk-win/      seul crate autorisé à contenir de l'`unsafe` (COM, D3D11, MF), chaque bloc avec // SAFETY:
  rk-core/     #![forbid(unsafe_code)] : horloge, ring buffer, pipeline, config, erreurs
  rk-mux/      #![forbid(unsafe_code)] : écriture MKV (EBML), entièrement testable hors Windows
  rk-app/      #![forbid(unsafe_code)] : tray, hotkeys, fenêtre de réglages, binaire
```
- Threads : capture (callback WGC) → encodage (événements MFT async) → mux/disque ; audio sur des threads WASAPI dédiés. Canaux bornés : si l'encodeur prend du retard, on **perd des images au lieu d'accumuler de la mémoire**, et on les compte.
- Horloge unique : QPC. Images et audio sont horodatés en QPC puis convertis en timebase MKV (1 ms ou ns).
- Replay buffer : anneau de **paquets déjà encodés** (pas d'images brutes), découpé par GOP. À la sauvegarde, on repart de l'image-clé qui précède `now - N s`. Mémoire estimée : débit × N, soit environ 750 Mo pour 50 Mb/s × 120 s **[S, calcul]**. Une limite stricte est configurable.
- Pas de réseau : aucune dépendance HTTP. `cargo deny` interdit les crates réseau connues.

## 4. Jalons

Un jalon par session (`/clear` entre deux). Chaque jalon se termine par `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check` et `cargo audit` au vert.

| # | Jalon | Critères d'acceptation mesurables | Test / bench |
|---|---|---|---|
| 1 | Squelette + CI | Workspace 4 crates ; CI GitHub Actions `windows-latest` (fmt, clippy, test, deny, audit) ; `forbid(unsafe_code)` actif hors `rk-win` | CI verte ; un test vérifie la présence de `forbid` |
| 2 | Capture → fichier (vidéo seule) | 1080p60 et 1440p60 pendant 10 min, encodeur matériel sans perte d'image côté app (compteur = 0) sur scène statique et en jeu ; repli logiciel fonctionnel si le matériel est absent ; fichier MKV lisible par `ffprobe` sans erreur ; écart de durée < 1 image | Test d'intégration (capture d'une fenêtre de test D3D11 animée, comptage des images) ; `ffprobe -v error` ; **bench d'overhead §5** + comparaison avec OBS |
| 3 | Muxer MKV robuste + remux MP4 | Fichier tué en cours d'enregistrement (`taskkill /F`) encore lisible jusqu'au dernier cluster écrit (perte ≤ 2 s) ; remux MP4 sans réencodage, durée identique ± 1 image | Tests unitaires EBML (round-trip via `matroska-demuxer`) ; test de crash scripté |
| 4 | Audio | Choix explicite de la sortie (loopback) et du micro ; 2 pistes AAC séparées ; dérive A/V < 1 image après 30 min ; périphérique débranché → enregistrement maintenu, piste en silence, événement journalisé | Test avec tonalité et flash synchronisés, mesure du décalage ; test de débranchement (manuel, documenté) |
| 5 | Replay buffer | Raccourci global → fichier des N dernières secondes (± 1 GOP) en < 1 s (taille du clip ≤ 500 Mo) ; mémoire bornée à la limite configurée ; aucune image perdue pendant la sauvegarde | Tests unitaires du ring (éviction, alignement GOP) ; mesure du délai de sauvegarde ; bench §5 avec buffer actif |
| 6 | UI | Tray (start/stop, sauver le replay, réglages, quitter) ; fenêtre de réglages ; config TOML validée (valeurs hors bornes refusées avec message) ; RAM au repos < 30 Mo **[cible, non mesurée]** | Tests de validation de config ; mesure de RAM au repos |
| 7 | Durcissement + packaging | Fuzzing du parseur de config et du muxer (`cargo fuzz`, 1 h sans crash) ; revue de chaque bloc `unsafe` ; `/security-review` ; build reproductible ; installeur ou zip portable + SBOM | Rapport de fuzzing ; SBOM (`cargo cyclonedx`) ; checklist de revue |

## 5. Méthode de benchmark de l'overhead

- **Outil FPS** : PresentMon (Intel, open source) en CLI. On mesure FPS moyen, 1 % low et 99e percentile du frametime du processus du jeu. **[S]** Licence et syntaxe CLI à vérifier au jalon 2.
- **Ressources** : compteurs PDH via `typeperf` (CPU du processus rekorder, `GPU Engine` 3D/VideoEncode, working set) échantillonnés à 1 s.
- **Charges** :
  1. Charge synthétique reproductible, versionnée dans le repo (`bench/gpu-load`, petit rendu D3D11 GPU-bound à FPS libre) : utilisable en CI locale.
  2. Un ou deux benchmarks intégrés de jeux (scène scriptée identique) pour la validation réelle.
- **Protocole** : machine branchée sur secteur, profil d'alimentation fixe, préchauffage de 2 min, puis 5 passes de 120 s pour chaque condition (sans enregistrement / enregistrement manuel / replay buffer / OBS mêmes réglages). On rapporte la médiane et l'écart min-max, ainsi que le delta en % par rapport à la référence sans enregistrement.
- **Résultats** : CSV bruts + script de synthèse dans `bench/results/<date>-<gpu>/`. Chaque rapport donne GPU, pilote, build Windows, résolution et débit.
- Aucun chiffre de performance n'est annoncé avant cette mesure.

## 6. Sécurité et licences

- `cargo deny` : licences autorisées MIT, Apache-2.0, BSD-2/3, Zlib, ISC, Unicode-3.0. GPL refusée. Avis RustSec bloquants.
- Le code `unsafe` reste confiné à `rk-win`, avec un commentaire `// SAFETY:` par bloc et `clippy::undocumented_unsafe_blocks` en deny.
- Chemins de sortie canonisés et restreints au dossier choisi ; noms de fichiers générés, jamais pris d'une entrée externe.
- Codecs : H.264, HEVC et AAC sont brevetés. Avec l'architecture A, on utilise les encodeurs fournis par Windows ou les pilotes, sans en embarquer. La portée des licences de brevets correspondantes est **[NV]** : ce n'est pas un avis juridique.

## 7. Risques

1. **Jeux plein écran exclusif / anti-cheat** : WGC peut ne pas capturer certains jeux en plein écran exclusif. Le hook D3D (méthode OBS « game capture ») est hors périmètre : il est intrusif et peut déclencher les anti-cheat. **[S]** Repli : capture d'écran.
2. **Bordure jaune WGC** en Win32 non packagé **[NV]** : pourrait imposer un packaging MSIX ou une bordure visible.
3. **Variabilité des MFT matériels** (réglages de débit, B-frames, AV1) selon NVIDIA, AMD et Intel ; je ne dispose que d'un seul GPU pour tester.
4. **RAM du replay buffer** à haut débit (voir §3).
5. **Muxer maison** : bugs de conteneur. Atténuation : round-trip, `ffprobe` et fuzzing.

## 8. Questions

Tranchées le 2026-10-20 (voir « Décisions validées » en tête).
