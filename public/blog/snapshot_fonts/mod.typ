#block(
  fill: rgb("#3d3522"),
  inset: 12pt,
  radius: 4pt,
  stroke: rgb("#5c4f33"),
)[
  #text(fill: rgb("#ffd866"))[*Note:*] This post was AI-generated from a #link("https://x.com/valera_other/status/2001352022877143142")[twitter thread].
]

#set text(font: "EB Garamond", size: 12pt)
#set heading(numbering: none)
#show heading.where(level: 1): set text(font: "Lato Light", size: 18pt, fill: rgb("#e07850"), weight: 200)
#show heading.where(level: 2): set text(font: "Lato Light", size: 14pt, fill: rgb("#e07850"), weight: 200)
#show link: set text(fill: rgb("#6eb5ff"))
#set par(justify: true, leading: 0.8em)

= Snapshot Fonts

#figure(
  image("./assets/main.jpg"),
  caption: [insta's `assert_snapshot!` for terminal chart rendering]
)

Terminal charts are visual output that needs deterministic testing. `assert_snapshot!` solves this - render, compare against stored reference, fail with diff if changed.

Snapshot tests make this trivial. Change rendering logic, run tests, see exactly what broke.
