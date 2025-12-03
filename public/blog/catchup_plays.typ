#block(
  fill: rgb("#fff3cd"),
  inset: 12pt,
  radius: 4pt,
  stroke: rgb("#ffecb5"),
)[
  *Note:* This post was AI-generated from a #link("https://x.com/valera_other/status/1993009688485806193")[twitter thread].
]

= Catchup Plays
or: how I mass-scalped my way into BTC at 87.5k and why it was "trivial"

== 0. the setup

Nov 24, 2025. BTC sitting around \$87,500. alts just got absolutely destroyed - ETH dropped 20%+ from recent highs, XRP got smacked 30%. the usual carnage.

but here's the thing: BTC was chilling. like, suspiciously chilling. just vibing near highs while its little brothers were getting their lunch money taken.

#figure(
  image("catchup_plays/assets/btc_heatmap.jpg"),
  caption: [BTC heatmap - aggregated across 13 exchanges. note the liquidity stacked above, price consolidating near highs]
)

#figure(
  image("catchup_plays/assets/eth_chart.png"),
  caption: [ETH getting rekt - from ~\$3,500 to ~\$2,700. that's the "extension"]
)

#figure(
  image("catchup_plays/assets/xrp_chart.png"),
  caption: [XRP even worse - \$2.55 to \$1.78. brutal]
)

the tweet that kicked it off:

#quote[trivial situation rn: major alts are showing significant extensions, can catch-up buy BTC]

== 1. why "trivial"?

look, it sounds arrogant. it's meant to. but there's actual logic here.

when correlated assets diverge significantly, one of two things happens:
1. the laggard catches up (alts bounce)
2. the leader catches down (BTC dumps)

either way, buying BTC here has decent RR because:
- if alts bounce hard, BTC probably goes with them (rising tide, all boats, etc)
- if BTC catches down, it's a slower process than the alt bloodbath - you have time to exit

the key insight: *the selling already happened*. just in the more volatile instruments. ETH has one of the highest long/short ratios on Binance. those longs got liquidated. the cascade that would normally drag BTC down already expressed itself elsewhere.

"Everyone was already liquidated and sold. Max pain is to run it back to 110k+"

== 2. first 10 minutes

#quote[was able to secure a BE stop on the position in like first 10m, just riding it now

I don't understand how I keep getting away with these

literally free money, how is this a thing]

this is the ideal scenario. enter, immediately get enough movement to move stop to breakeven. now you're playing with house money.

the incredulity is half-genuine. these setups feel like cheating when they work. (they don't always work.)

== 3. the sell wall

#figure(
  image("catchup_plays/assets/sell_wall.png"),
  caption: [someone really wants to sell their 1k BTC around \$88,000]
)

#quote[what's this now, someone really wants to sell their 1k BTC

very impatient too, fascinatingly

pretty sure we'll eat through it (major alts momentum is overwhelming), but probably will have to start reducing certainty if this guy doubles down]

this is where most people panic. big wall appears, must mean price going down, right?

wrong. or at least, not necessarily. the wall tells you there's a seller. it doesn't tell you if there are enough buyers to absorb it. in this case, alts were ripping - XRP +14%, ADA +6%, ETH running. that momentum had to go somewhere.

== 4. wall gets eaten

#quote[hell yeah, suck it non-believer

should've been a bull, ye of little faith]

this is the fun part. wall disappeared. whoever was selling either got filled or pulled their order. either way: obstacle removed.

== 5. position management (the actual alpha)

#quote[talking of faith, - I don't have any. First tp (10%)]

immediately after celebrating the wall getting eaten: take profit. 10% of the position.

this is counterintuitive to most. "but it's working! why sell?"

because:
- you lock in gains
- you reduce exposure as uncertainty increases
- you remove the psychological weight of the full position

the solution to getting caught wrong isn't better prediction - it's incremental position changes. much easier to dispose of 10% every time you realize you're "wronger than expected" than struggling with all-or-nothing decisions.

== 6. resistance incoming

#figure(
  image("catchup_plays/assets/88100_resistance.jpg"),
  caption: [approaching \$88,100 resistance. CVD, depth ratio, liquidations, OI - the full dashboard]
)

#quote[20% closed going into 88100, - will have a significant reaction to the recent resistance, or I don't wanna play anymore]

another 10% off (20% total now). why? resistance level approaching. might blast through, might reject hard. either way, reduce size before finding out.

note the dashboard:
- CVD (cumulative volume delta) at -6.06B
- OB depth ratio slightly negative
- liquidations calming down
- OI around 2.64B

all useful context, none of it predictive by itself. you're pattern matching across multiple signals.

== 7. reading the momentum fade

#figure(
  image("catchup_plays/assets/alts_calming.jpg"),
  caption: [alts starting to calm down - the momentum engine is losing steam]
)

#figure(
  image("catchup_plays/assets/alts_calming2.jpg"),
  caption: [XRP +14%, ADA +6%, ETH following. but the rate of change is slowing]
)

#quote[fine, I'll entertain it a bit longer

alts are starting to calm down though, so it's just the question of squeezing a few extra bucks with slightly better tp levels;
rather than even entertaining holding the position]

this is key. the whole thesis was "alts extended, catching up". now they've caught up significantly. the momentum engine that was driving the BTC bid is losing steam.

at this point you're not trading conviction anymore. you're squeezing residual value from a position that's served its purpose.

== 8. probabilistic exit

#quote[actually, might be some value in reaching for 90k, - think these can keep going by themselves in ~38% of the cases, so RR just makes sense

put a momentum-based stop for remaining 60%, - it can do whatever now. I'mma go back to coding]

62% chance it dies here, 38% it runs to 90k. the remaining 60% of position has a momentum-based stop. if it keeps running, great. if momentum fades, auto-exit.

this is the correct way to handle uncertainty: assign rough probabilities, size accordingly, automate the exit logic, go do something else.

== 9. spidey senses

#figure(
  image("catchup_plays/assets/spidey_senses.png"),
  caption: [the momentum stop triggered. red arrows = partial TPs throughout the move]
)

#quote[spidey senses were tingling]

momentum faded, stop triggered, position closed. the red arrows on the chart show the partial TPs taken throughout the move.

== 10. what actually happened here

let's break down the decision tree:

*entry logic:*
- alts showed "extensions" (major relative drawdowns)
- BTC holding firm near highs
- selling pressure already expressed in alts
- orderbook favorable (liquidity above, not below)

*position management:*
- immediate BE stop (risk removal)
- 10% TP on first push (lock gains)
- 10% more into resistance (reduce before uncertainty)
- momentum-based stop on remainder (let winners run, cut losers)

*exit logic:*
- alt momentum fading = thesis weakening
- resistance approaching = uncertainty increasing
- probabilistic assessment: 38% chance of 90k, so RR still there for reduced size
- automated stop handles the rest

== 11. the meta-lesson

these "trivial" setups are trivial *in hindsight* and *if you know what to look for*. the actual skill is:

1. recognizing the divergence in real-time
2. having the orderflow tools to validate the setup
3. managing position incrementally, not all-or-nothing
4. knowing when your thesis is weakening
5. automating exits so emotions don't interfere

the position sizing advice from a related thread: "much easier to dispose of ~10% pos every time you realize you're wronger than expected, than struggling with the weight of all-or-nothing decisions. same for tps."

and the psychological trap to avoid:

#quote[> "okay I've learned my lesson please just get me back to breakeven bro please"

*gets back to breakeven*

> "lmfao can't believe I was freaking out over nothing. we're going so much higher"

*market dies again*]

the catchup play exploits this cycle. you're entering when others are begging for breakeven. you're taking profit when they've convinced themselves the pain was "nothing."

== 12. tools used

- MMT PRO for aggregated heatmaps (13 exchanges)
- TradingView for alt charts
- VPSV indicators (London/NewYork/Asia sessions)
- CVD, OB depth ratio, liquidations, OI panels
- custom momentum-based stop logic

the tools matter less than the framework. you need *some* way to see:
- relative performance across correlated assets
- orderbook depth and flow
- momentum/rate of change

== 13. final thoughts

"I don't understand how I keep getting away with these"

you don't "get away with" anything. you identify setups with favorable RR, size appropriately, manage incrementally, and accept that you'll be wrong plenty. the wins compound because you're not giving back gains on the losers.

the "trivial" framing is partly tongue-in-cheek, partly earned confidence from pattern recognition. after seeing enough alt extensions lead to BTC continuation, the setup becomes obvious.

until it doesn't work. then you're glad you had that BE stop and those partial TPs.

literally free money. how is this a thing.
