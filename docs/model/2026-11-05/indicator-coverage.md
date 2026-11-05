# Indicator Coverage — 10 OHLCV Information Areas

- Date: 2026-11-05 (UTC)
- Status: **Implemented** — 46 indicators, 143 events, 110 parameters; event codes 1–143; 37/37 checklist rows `verified`; spec v2 implemented; validation claims open
- Scope: 10 information areas, 36 new indicators, 10 existing indicators
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Retrospective audit: [`docs/improvement/2026-11-05/INDICATOR-AUDIT.md`](../../improvement/2026-11-05/INDICATOR-AUDIT.md)
- M2.x programme: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Contract: `AGENTS.md`, `docs/model/2026-11-04/model-v4.md`, `docs/registry/README.md`
- Claim type: registry and indicator contract; no edge claim

## 1. Completeness rule

An area scores **10/10** when it has:

1. at least three orthogonal readings (existing readings count only where they
   cover the area);
2. at least one level, one divergence, and one signal event where the
   mathematics is meaningful;
3. a role, a falsifiable claim, and a math formula for every event;
4. causality, determinism, golden, and edge-case tests for every reading and
   event;
5. no dependency outside OHLCV and time.

Some areas have events that are mathematically impossible or meaningless (for
example, a calendar bucket has no divergence). Those are marked `none` and the
area still scores 10/10 because the remaining event types and the role are
covered. The registry never invents a meaningless event to fill a cell.

## 2. Notation

- `O,H,L,C,V` — open, high, low, close, volume at bar `t`.
- `TP = (H+L+C)/3`.
- `ATR = ATR14`.
- `mean(x,n)`, `sum(x,n)` — over the last `n` bars including `t`.
- `sign(x)` — `-1`, `0`, `+1`.
- `abs(x)`.
- `cross_up(a,b)` — `a[t] > b[t] && a[t-1] <= b[t-1]`; `cross_down` mirror.
- `test(x,L)` — `abs(x-L) <= 0.2*ATR`.
- `p` — the previous confirmed major pivot bar.
- `RV5, RV20, RV50` — `V/mean(V,n)`.
- Division by zero or a missing denominator publishes `null`, never zero.

## 3. Area 1 — Volume-weighted price (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 11 | `vwap` | `TP=(H+L+C)/3`; `VWAP=sum(TP*V,n)/sum(V,n)`; `D=(C-VWAP)/ATR`; `S=VWAP-VWAP[n]`; `MAD=mean(abs(TP-VWAP),n)`; bands `VWAP±2MAD` | `test(C,VWAP)`; `cross_up(C,VWAP)`; `C>VWAP+0.2ATR && C[t-1]>VWAP[t-1]+0.2ATR` | bearish: `H[t]>H[p] && D[t]<D[p]`; bullish: `L[t]<L[p] && D[t]>D[p]`; hidden mirrors | reclaim: `cross_up(C,VWAP) && count(C<VWAP,6)>=6`; rejection: `H>=VWAP && C<VWAP && C<O`; slope: `sign(S[t])!=sign(S[t-1])` | context/signal |

## 4. Area 2 — Volume by price (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 12 | `volume_profile` | 96 bars, 24 bins over `[min L,max H]`; `bin_j=sum(V_i where TP_i in bin_j)`; `POC=argmax(bin)`; `VA` expands from POC until `sum(bin)>=0.7*sum(V)`; `PD=(C-POC)/ATR`; `VP=(C-VAL)/(VAH-VAL)`; `SK=(sum V above POC - sum V below POC)/sum V`; `CO=max(bin)/sum(V)`; `PM=POC-POC[n]` | `test(C,POC)`; `test(H,VAH)`; `test(L,VAL)`; `C>VAH` or `C<VAL`; re-entry `C[t-1]>VAH && C[t]<=VAH` | bearish: `H[t]>H[p] && PM[t]<PM[p]`; bullish: `L[t]<L[p] && PM[t]>PM[p]` | `C>VAH && RV20>=1.5`; `cross_up(C,POC)`; `sign(SK[t])!=sign(SK[t-1])`; `cross_up(CO,0.5)` | context/signal |

## 5. Area 3 — Participation (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 4 | `rvol` (extended) | `RV5=V/mean(V,5)`; `RV20=V/mean(V,20)`; `RV50=V/mean(V,50)`; `RR=RV5/RV50` | none | bearish: `H[t]>H[p] && RV50[t]<RV50[p]`; bullish mirror | `cross_up(RV5,1.5)`; `cross_down(RV5,0.7)`; `cross_up(RR,1.5)` | context/signal |
| 13 | `volume_climax` | `CL=RV5>=3 && (H-L)/ATR>=1.5`; `CD=sign(C-O)`; `CS=RV5*(H-L)/ATR` | none | bearish: `H[t]>H[p] && RV5[t]<RV5[p]`; bullish mirror | reversal: `CL && C inside [L[t-1],H[t-1]]` → `-CD`; continuation: `CL && CP>=0.8` → `+1` | signal |
| 14 | `absorption` | `AB=RV5>=2 && (H-L)/ATR<=0.6 && min(|C-VWAP|,|C-POC|)<=0.25ATR`; `AS=RV5/((H-L)/ATR)` | `AB` at a level | none | `C>H_abs+0.2ATR` → +1; `C inside absorption range` → `-AS_side` | signal/context |
| 15 | `volume_oscillator` | `VO=(mean(V,5)-mean(V,20))/mean(V,20)` | none | none | `cross_up(VO,0)`; `cross_down(VO,0)` | context |

## 6. Area 4 — Volume-price flow (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 16 | `obv` | `OBV[t]=OBV[t-1]+sign(C[t]-C[t-1])*V[t]`; `OBS=OBV-OBV[n]`; `OBM=mean(OBV,n)` | none | bearish: `H[t]>H[p] && OBV[t]<OBV[p]`; bullish mirror | `cross_up(OBV,OBM)`; `cross_down(OBV,OBM)` | context/signal |
| 17 | `cmf` | `MFV=((C-L)-(H-C))/(H-L)*V`; `CMF=sum(MFV,n)/sum(V,n)` | none | bearish: `H[t]>H[p] && CMF[t]<CMF[p]`; bullish mirror | `cross_up(CMF,0)`; `abs(CMF)>=0.2` | context/signal |
| 18 | `mfi` | `TP=(H+L+C)/3`; `POS=sum(TP*V where TP>TP_prev)`; `NEG=sum(TP*V where TP<TP_prev)`; `MFI=100-100/(1+POS/NEG)` | none | bearish: `H[t]>H[p] && MFI[t]<MFI[p]`; bullish mirror | `cross_up(MFI,20)`; `cross_up(MFI,50)`; `cross_up(MFI,80)` | signal |
| 19 | `ad_line` | `AD[t]=AD[t-1]+MFV`; `ADM=mean(AD,n)` | none | bearish: `H[t]>H[p] && AD[t]<AD[p]`; bullish mirror | `cross_up(AD,ADM)`; `cross_down(AD,ADM)` | context |

## 7. Area 5 — Volatility structure (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 20 | `atr_term` | `R=ATR14/ATR100`; `S=R-R[n]`; `REG=0 if R<=0.7, 1 if 0.7<R<1.3, 2 if R>=1.3` | none | bearish: `H[t]>H[p] && R[t]<R[p]`; bullish mirror | `R<=0.7`; `cross_up(R,1.3)`; `REG[t]!=REG[t-1]` | context/signal |
| 21 | `range_expansion` | `RR=(H-L)/mean(H-L,n)` | none | none | `cross_up(RR,1.5)`; `cross_down(RR,0.7)` | context/signal |
| 22 | `mar` | `MAR=mean(abs(C-C_prev)/C_prev,n)`; `MR=MAR5/MAR20` | none | none | `cross_up(MR,1.5)`; `cross_down(MR,0.7)` | diagnostic |
| 23 | `keltner` | `K=EMA(C,20)`; `U=K+2ATR`; `L=K-2ATR`; `KP=(C-L)/(U-L)` | `test(C,U)`; `C>U` break | none | `C>U`; `C<L`; `cross_up(KP,0.8)`; `cross_down(KP,0.2)` | context/signal |

## 8. Area 6 — Structure (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 6 | `pivots` (existing) | confirmed high/low at 2 minor and 5 major bars; available at `t+k` | `test(C,L)`; `cross_up(C,L)`; `C>L` | price vs RSI/OBV at pivots | `SS` change | signal/context |
| 24 | `donchian` | `DH=max(H,n)`; `DL=min(L,n)`; `DP=(C-DL)/(DH-DL)` | `test(H,DH)`; `C>DH` | none | `C>DH && RR>=1.2`; `C<DL`; `cross_up(DP,0.8)` | context/signal |
| 25 | `swing_structure` | `HH=H[p]>H[p-1]`; `HL=L[p]>L[p-1]`; `SS=+1 if HH&&HL, -1 if LH&&LL, 0 otherwise` | `C>last_pivot_high`; `C<last_pivot_low` | none | `SS[t]!=SS[t-1]` | context |
| 26 | `level_strength` | `T=count(test(H,L) or test(L,L),n)`; `S=T/n` | `T>=2` test/break | none | `cross_up(S,0.5)` | context |
| 27 | `aroon` | `AU=100*(n-bars_since_high)/n`; `AD=100*(n-bars_since_low)/n`; `AO=AU-AD` | none | none | `cross_up(AU,70)`; `cross_down(AD,30)`; `cross_up(AO,0)` | context |
| 28 | `bos_choch` | `BOS=C>last_pivot_high in up structure`; `CHOCH=C>last_pivot_high in down structure`; mirrors | BOS/CHOCH event | none | `SS` direction change | signal/context |

## 9. Area 7 — Candle anatomy and gaps (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 29 | `candle_anatomy` | `R=H-L`; `B=abs(C-O)`; `UW=H-max(O,C)`; `LW=min(O,C)-L`; `CP=(C-L)/R`; `BR=B/R`; `UWR=UW/R`; `LWR=LW/R` | `UWR>=0.6 && test(H,L)`; `LWR>=0.6 && test(L,L)` | none | `R>=1.5ATR && BR>=0.7 && CP>=0.8`; `UWR>=0.6 && C<O`; `BR<=0.1`; `BR>=0.9` | context/signal |
| 30 | `candle_pattern` | engulfing: `C>O && O<=C_prev && C>=O_prev`; inside: `H<=H_prev && L>=L_prev`; outside: `H>H_prev && L<L_prev`; hammer: `LW>=2B && UW<=0.1R && CP>=0.7`; star: `UW>=2B && LW<=0.1R && CP<=0.3` | none | none | pattern flags | context/signal |
| 31 | `gap` | `G=O-C_prev`; `GA=G/ATR` | fill: `C` returns to `C_prev`; `abs(G)<=0.2ATR` | none | continuation: `GA>=0.5 && C>O`; fade: `GA>=0.5 && C<C_prev` | context/signal |

## 10. Area 8 — Path and duration (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 32 | `path_efficiency` | `ER=abs(C-C_n)/sum(abs(C_i-C_{i-1}),n)` | none | none | `cross_up(ER,0.5)`; `cross_down(ER,0.2)` | diagnostic |
| 33 | `direction_changes` | `DC=sum(sign(C_i-C_{i-1})!=sign(C_{i-1}-C_{i-2}),n)`; `DR=DC/n` | none | none | `cross_up(DR,0.5)`; `cross_down(DR,0.3)` | diagnostic |
| 34 | `bars_since_extreme` | `BH=n-index(max H,n)`; `BL=n-index(min L,n)` | none | none | `BH==0` new high; `BH>=n-1` stale | context |
| 35 | `drawdown_atr` | `DD=(C-max(H,n))/ATR` | none | none | `cross_down(DD,-1)`; `cross_down(DD,-2)`; `cross_up(DD,-0.5)` | context |
| 36 | `range_position` | `RP=(C-min(L,n))/(max(H,n)-min(L,n))` | none | none | `cross_up(RP,0.8)`; `cross_down(RP,0.2)` | context |

## 11. Area 9 — Cross-asset (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 37 | `relative_strength` | `ret_i=(C_i-C_i[n])/ATR_i`; `RS=ret_i-mean(ret_j)` | none | bearish: `H[t]>H[p] && RS[t]<RS[p]`; bullish mirror | `cross_up(RS,0)`; `cross_down(RS,0)` | context/signal |
| 38 | `lead_lag` | `lead=ret_leader[t-k]`; `follow=ret_follower[t]`; `LL=lead-follow` | none | `sign(lead)!=sign(follow) && abs(lead)>=1` | `cross_up(lead,1)`; `cross_down(lead,-1)` | signal/context |
| 39 | `breadth` | `BR=count(C_i>VWAP_i or ret_i>0)/N` | none | bearish: `H[t]>H[p] && BR[t]<BR[p]`; bullish mirror | `cross_up(BR,0.67)`; `cross_down(BR,0.33)` | context |
| 40 | `co_movement` | `CM=sum(sign(ret_i)==sign(ret_basket))/n` | none | none | `cross_up(CM,0.7)`; `cross_down(CM,0.3)` | diagnostic |
| 41 | `dispersion` | `D=max(ret_j)-min(ret_j)` | none | none | `cross_up(D,threshold)` | diagnostic |

## 12. Area 10 — Calendar and session (10/10)

| Code | Indicator | Reading formula | Level event | Divergence event | Signal event | Role |
|---:|---|---|---|---|---|---|
| 42 | `session` | DST-aware session code; `SA=bars since open`; `SVWAP` from open | `test(C,SVWAP)` | none | `SA==0` session open; last session bar, available next bar, session close | context |
| 43 | `opening_range` | `ORH=max(H,first k)`; `ORL=min(L,first k)`; `ORR=(ORH-ORL)/ATR` | `test(H,ORH)`; `C>ORH` | none | after k bars: `C>ORH` breakout; `C<ORL` fade | context/signal |
| 44 | `overnight_intraday` | `ON=O_session-C_prev`; `ID=C-O_session` | none | none | at session open: `ON>=0.5ATR && ID>0` continuation; `ON>=0.5ATR && ID<0` fade | signal/context |
| 45 | `session_range` | `SR=(session_high-session_low)/ATR` | none | none | `cross_up(SR,1.5)`; `cross_down(SR,0.7)` | context |
| 46 | `calendar_bucket` | `DOW=0..6`; `MON=1..12` | none | none | none | diagnostic |

## 13. Completeness matrix

| Area | Indicators | Level | Divergence | Signal | Roles | Score |
|---|---|---:|---:|---:|---|---:|
| Volume-weighted price | 1 | yes | yes | yes | context/signal | **10/10** |
| Volume by price | 1 | yes | yes | yes | context/signal | **10/10** |
| Participation | 4 | yes | yes | yes | context/signal | **10/10** |
| Volume-price flow | 4 | no | yes | yes | context/signal | **10/10** |
| Volatility structure | 4 | yes | yes | yes | context/signal | **10/10** |
| Structure | 6 | yes | yes | yes | context/signal | **10/10** |
| Candle anatomy / gaps | 3 | yes | no | yes | context/signal | **10/10** |
| Path / duration | 5 | no | no | yes | context/diagnostic | **10/10** |
| Cross-asset | 5 | no | yes | yes | context/signal | **10/10** |
| Calendar / session | 5 | yes | no | yes | context/signal | **10/10** |

Areas with `no` for an event type have no mathematically meaningful event of
that type; the registry marks it `none` rather than inventing one. The area
still has a reading, a role, a claim, and tests.

## 14. Registry code map

### 14.1 Indicator codes

Existing: 1 `ema`, 2 `rsi`, 3 `atr`, 4 `rvol`, 5 `macd`, 6 `pivots`, 7
`context`, 8 `progression`, 9 `adx`, 10 `dmi`.

New: 11 `vwap`, 12 `volume_profile`, 13 `volume_climax`, 14 `absorption`, 15
`volume_oscillator`, 16 `obv`, 17 `cmf`, 18 `mfi`, 19 `ad_line`, 20 `atr_term`,
21 `range_expansion`, 22 `mar`, 23 `keltner`, 24 `donchian`, 25
`swing_structure`, 26 `level_strength`, 27 `aroon`, 28 `bos_choch`, 29
`candle_anatomy`, 30 `candle_pattern`, 31 `gap`, 32 `path_efficiency`, 33
`direction_changes`, 34 `bars_since_extreme`, 35 `drawdown_atr`, 36
`range_position`, 37 `relative_strength`, 38 `lead_lag`, 39 `breadth`, 40
`co_movement`, 41 `dispersion`, 42 `session`, 43 `opening_range`, 44
`overnight_intraday`, 45 `session_range`, 46 `calendar_bucket`.

### 14.2 Source codes

Existing 1–18 are preserved. New: 19 `vwap`, 20 `volume_profile`, 21
`volume_climax`, 22 `absorption`, 23 `volume_oscillator`, 24 `obv`, 25 `cmf`,
26 `mfi`, 27 `ad_line`, 28 `atr_term`, 29 `range_expansion`, 30 `mar`, 31
`keltner`, 32 `donchian`, 33 `swing_structure`, 34 `level_strength`, 35
`aroon`, 36 `bos_choch`, 37 `candle_anatomy`, 38 `candle_pattern`, 39 `gap`,
40 `path_efficiency`, 41 `direction_changes`, 42 `bars_since_extreme`, 43
`drawdown_atr`, 44 `range_position`, 45 `relative_strength`, 46 `lead_lag`,
47 `breadth`, 48 `co_movement`, 49 `dispersion`, 50 `session`, 51
`opening_range`, 52 `overnight_intraday`, 53 `session_range`, 54
`calendar_bucket`.

### 14.3 Signal subtypes

Existing 1–9 are preserved. New: 10 candle, 11 participation, 12 absorption,
13 volume profile, 14 VWAP, 15 volume flow, 16 volatility, 17 range, 18
channel, 19 structure, 20 gap, 21 path, 22 cross-asset, 23 session.

## 15. Registry entry shape

Every indicator entry must carry:

```json
{
  "id": "vwap",
  "code": 11,
  "name": "Volume-weighted average price",
  "taxonomy": "volume",
  "role": "context",
  "claim": "A close above the rolling VWAP ...",
  "formula": "TP=(H+L+C)/3; VWAP=sum(TP*V,n)/sum(V,n); D=(C-VWAP)/ATR",
  "inputs": ["high", "low", "close", "volume", "atr"],
  "parameters": [{"id": "vwap.window", "value": 24, "type": "integer"}],
  "outputs": ["vwap", "vwap_distance_atr", "vwap_slope", "vwap_band_upper", "vwap_band_lower"],
  "availability": "n bars and sum(V,n) > 0",
  "events": [
    {
      "id": "level.vwap_test",
      "event_type": 1,
      "subtype": 1,
      "source": 19,
      "role": "context",
      "claim": "none",
      "formula": "abs(C-VWAP) <= 0.2*ATR",
      "parameters": ["events.level.test_atr"],
      "precedence": [],
      "tests": ["causality", "determinism", "golden", "edge"]
    }
  ],
  "tests": ["causality", "determinism", "golden", "edge"],
  "status": "planned"
}
```

Every event entry must carry a `formula` string. `tools/check_registry.py` must
reject an event without a formula.

## 16. Test plan

For every reading:

| Test | What it proves |
|---|---|
| `formula` | A hand-computed synthetic series matches the output |
| `causality` | Appending future bars does not change past values |
| `determinism` | Two runs with the same input produce identical output |
| `golden` | A checked-in fixture matches the current output |
| `edge` | Empty, one bar, insufficient history, zero range, zero volume, divide by zero, equal highs/lows |

For every event:

| Test | What it proves |
|---|---|
| `availability` | The event is not published before `available_bar` |
| `no_double_count` | One underlying occurrence is emitted once |
| `precedence` | Level/signal ordering is fixed and documented |
| `direction` | The direction code matches the formula |
| `formula` | A hand-computed synthetic case matches the event |

For every spec:

| Test | What it proves |
|---|---|
| `conformance` | The spec reproduces a known fixture |
| `determinism` | Artifacts are byte-identical across runs |
| `registry` | Codes, roles, claims, and formulas are valid |

## 17. Deliberately excluded

| Excluded | Why |
|---|---|
| Stochastic, Williams %R, CCI | Price-momentum transforms, collinear with RSI |
| Bollinger Bands with standard deviation | Uses a statistic and a square root; replaced by Keltner (ATR) and VWAP MAD bands |
| Ichimoku, Parabolic SAR, Supertrend | Price/trend transforms, collinear with EMA/ADX |
| Fibonacci levels | Geometric price levels; no new information; excluded by the model contract |
| Correlation, beta, regression | Statistics/fitted coefficients; replaced by co-movement and dispersion (sign/range arithmetic) |
| Tick, order book, funding, open interest, news | Outside OHLCV scope |

## 18. Execution phases

| Phase | Work | Gate |
|---|---|---|
| 0 | Registry schema extension and the 46-indicator table | Human approves definitions and formulas |
| 1 | Read-only feature audit of the top 10 | At least one survivor or stop |
| 2 | Tier 1 readings: VWAP, profile, ATR term, participation, lead-lag/breadth, Donchian | Causality, determinism, golden, edge |
| 3 | Event layer: level, divergence, signal, precedence | Event tests |
| 4 | Spec engine: schema, evaluator, `--spec`, registry, conformance | `cargo test`, clippy, fmt |
| 5 | Tier 2 readings: flow, path, cross-asset context, calendar | Tests |
| 6 | M2.x charters and swarm | Human approves charters |
| 7 | Validation and baseline | Quant Validator, human acceptance |
| 8 | Closeout | Registry, index, baselines, cleanup |

## 19. Stop conditions

- A formula cannot be expressed with the allowed arithmetic.
- A reading or event fails causality, determinism, golden, or edge tests.
- The feature audit finds no candidate above the screen.
- An M2.x premise is a variant of a failed hypothesis.
- No cell passes dev; validation fails; human stops.
