# Mountain Mining Protocol (MMP) — Proje Bağlamı

> Bu dosya GitHub Copilot'a (veya repoyu devralan başka bir yapay zekaya) projeyi tanıtmak
> için yazıldı. Copilot bu tür dosyaları otomatik okuyup bağlam olarak kullanır.

## 1. Proje Nedir

Mountain Mining Protocol, **Solana/Anchor** üzerinde çalışan, NFT tabanlı pasif bir madencilik
(mining) protokolü. 100.000 adet "Mining Pass" NFT'si, sabit 1.000.000.000 (1 milyar) arzlı bir
**MMP** token'ını 20 yıllık bir süreye yayarak madenciliğe açıyor.

NFT bir finansal getiri vaadi değil; sadece madencilik sistemine erişim hakkı.

## 2. Teknoloji Yığını

- **On-chain program:** Rust + Anchor framework (Solana)
- **Token standardı:** Klasik SPL Token (Token-2022 değil)
- **NFT metadata:** Metaplex Token Metadata — koleksiyon doğrulaması dependency-free şekilde,
  elle yazılmış mantıkla (`metaplex.rs`)
- **Frontend:** Next.js + TypeScript, `@solana/wallet-adapter-*`
- **Test:** Anchor/Mocha entegrasyon testleri + saf Rust `cargo test` reward-math testleri

## 3. Token Ekonomisi (MMP)

| Parametre | Değer |
|---|---|
| Toplam MMP arzı | 1.000.000.000 MMP (sabit tavan) |
| Mint/burn/pause/admin backdoor | Yok — tamamen trust-minimized |
| Mining süresi (protokol geneli hedef) | 20 yıl = 630.720.000 saniye |
| Mining Pass NFT toplam adedi | 100.000 |
| Sınıf sayısı | 7 (Stone, Obsidian, Iron, Steel, Titanium, Diamond, Mithril) |
| Toplam ağırlıklı güç (TOTAL_POWER) | 486.000 |

### Sınıf tablosu

| Sınıf | Adet | Çarpan | Adet × Çarpan |
|---|---|---|---|
| Stone | 40.000 | 1× | 40.000 |
| Obsidian | 25.000 | 2× | 50.000 |
| Iron | 15.000 | 4× | 60.000 |
| Steel | 10.000 | 8× | 80.000 |
| Titanium | 6.000 | 16× | 96.000 |
| Diamond | 3.000 | 32× | 96.000 |
| Mithril | 1.000 | 64× | 64.000 |
| **Toplam** | **100.000** | — | **486.000** |

Bu tablo on-chain'de değiştirilemez bir sabit (`CLASS_TABLE`) olarak tutulur ve derleme
zamanında toplamların doğruluğu otomatik doğrulanır.

### NFT dağıtımı (3 faz)

- 10.000 ücretsiz airdrop
- 10.000 erken erişim
- 80.000 halka açık satış

Sınıf ataması alıcı tarafından seçilemez — protokol tarafından (verifiable randomness ile)
atanır.

## 4. Ödül Formülü

20 yıl, tüm 486.000 birimlik gücün kesintisiz madencilik yaptığı teorik maksimum senaryoda
Mountain'ın toplam ömrü için bir hedef/asgari süredir — belirli bir NFT için zorunlu bir bitiş
tarihi değildir. Madencilik oturumları süresiz durdurulup yeniden başlatılabilir; kümülatif
muhasebe ile takip edilir.

```
reward = floor(
    elapsed_seconds * class_multiplier * TOTAL_SUPPLY
    / (MINING_PERIOD_SECONDS * TOTAL_POWER)
)
```

Tüm ara çarpımlar `u128` ile hesaplanır, floating point kullanılmaz.

Global 1 milyar MMP tavanı, claim anında `min(calculated_reward, remaining_supply)` ile
uygulanır — hesaplanan ödül kalan arzdan fazlaysa işlem revert olmaz, sadece kalan arz kadar
mint edilir.

## 5. Madencilik Yaşam Döngüsü

```
NFT SAHİPLİĞİ → MINE (NFT program custody'sine kilitlenir, MiningState PDA açılır/yenilenir)
    → PASİF MADENCİLİK (zaman damgasından hesaplanır)
    → CLAIM (tekrarlanabilir)
    → STOP (otomatik settle + NFT sahibine geri döner)
```

STOP komutu otomatik "settle" yapar: NFT'yi kilitten çıkarmadan önce, biriken ödülü otomatik
mint edip kullanıcıya gönderir.

NFT, madencilik sırasında programın kontrolündeki bir escrow/custody hesabına taşınır (PDA
tarafından yönetilen token hesabı).

## 6. Güvenlik Mimarisi

- Hiçbir admin/owner anahtarı MMP mint edemez, kullanıcı bakiyesini değiştiremez, NFT çalamaz
  veya claim'i override edemez.
- MMP mint authority'si sadece bir program PDA'sı.
- On-chain class-capacity enforcement: `ProtocolConfig` hesabında her sınıf için canlı bir
  sayaç tutulur, mint sırasında sadece kalan kapasitesi olan sınıflar arasından seçim yapılır.
- Program ID, gerçek bir ed25519 keypair ile `declare_id!` ve `Anchor.toml` içine yazılıdır —
  ama bu **deploy öncesi yeniden üretilmesi gereken**, geliştirme ortamı kökenli bir anahtardır.

## 7. Program Hesapları (PDA'lar)

- `ProtocolConfig` (singleton): global durum — toplam mint edilen MMP, faz sayaçları, sınıf
  başına kalan kapasite sayaçları, mint authority bump'ları
- `MiningState` (her NFT için, `[b"mining_state", pass_mint]`): sahip, sınıf ID, mining durumu,
  mining_start, last_claim_ts, lifetime_claimed
- Mint authority PDA (`[b"mint_authority"]`): sadece imzalama yetkisi
- Pass custody PDA (`[b"custody", pass_mint]`): madencilik sırasında NFT'yi tutan escrow
  hesabının otoritesi

## 8. Yapılmaması Gerekenler

Staking yok, governance yok, DAO yok, referral ödülü yok, ikinci bir token yok, keyfi
vergi/gizli ücret yok, gizli admin yetkisi yok, upgrade backdoor yok. **Mainnet'e deploy YOK** —
proje bilinçli olarak devnet-ready seviyesinde tutuluyor.

## 9. Bilinen Sınırlamalar

- Sınıf atamasındaki rastgelelik kaynağı (`SlotHashes` tabanlı) production-grade bir VRF
  değildir; mainnet öncesi Switchboard/ORAO gibi bir oracle VRF ile değiştirilmesi gerekir.
- Frontend'in mine/claim/stop butonları gerçek bir `anchor build` IDL dosyasına bağlıdır.
- Program ID sandbox kökenlidir, gerçek deploy öncesi yeniden üretilmelidir (`anchor keys sync`).

## 10. Copilot'tan Beklenen Yaklaşım

- Ekonomik sabitleri (arz tavanı, sınıf tablosu, 20 yıllık süre, faz adetleri) asla sessizce
  değiştirme.
- Ödül matematiğinde floating point kullanma; `u128` ara hesaplama + `checked_*` operasyonları
  kullan.
- Yeni bir admin yetkisi, upgrade yolu veya gizli mint fonksiyonu ekleme.
- Var olan güvenlik kısıtlamalarını gevşetme; her yeni instruction için aynı düzeyde
  sahiplik/durum/tavan kontrolü ekle.
