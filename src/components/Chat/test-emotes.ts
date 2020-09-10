export interface EmoteVersion {
  url?: string;
  path: string;
  animated: boolean;
  dimensions: {
    height: number;
    width: number;
  };
  size: "1x" | "2x" | "4x";
}

export interface Emote {
  name: string;
  versions: EmoteVersion[];
}

export const emotes: Emote[] = [
  {
    "name": "INFESTOR",
    "versions": [
      {
        "path": "img/emotes/INFESTOR.3fdbf6.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/INFESTOR.b44af1.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/INFESTOR.11a1e5.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FIDGETLOL",
    "versions": [
      {
        "path": "img/emotes/FIDGETLOL.299921.png",
        "animated": false,
        "dimensions": { "height": 25, "width": 25 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FIDGETLOL.09f6c9.png",
        "animated": false,
        "dimensions": { "height": 50, "width": 50 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FIDGETLOL.cc9567.png",
        "animated": false,
        "dimensions": { "height": 100, "width": 100 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Hhhehhehe",
    "versions": [
      {
        "path": "img/emotes/Hhhehhehe.411b11.png",
        "animated": false,
        "dimensions": { "height": 25, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Hhhehhehe.ac8c74.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Hhhehhehe.08c867.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "GameOfThrows",
    "versions": [
      {
        "path": "img/emotes/GameOfThrows.7a83dd.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 79 },
        "size": "1x",
      },
      {
        "path": "img/emotes/GameOfThrows.8c9051.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 158 },
        "size": "2x",
      },
      {
        "path": "img/emotes/GameOfThrows.08cfcc.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 316 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Abathur",
    "versions": [
      {
        "path": "img/emotes/Abathur.991741.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 84 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Abathur.143958.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 168 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Abathur.99e315.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 336 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "LUL",
    "versions": [
      {
        "path": "img/emotes/LUL.5a8555.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/LUL.91fb2f.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/LUL.6266e3.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SURPRISE",
    "versions": [
      {
        "path": "img/emotes/SURPRISE.ace947.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SURPRISE.723284.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 66 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SURPRISE.e6a366.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 132 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NoTears",
    "versions": [
      {
        "path": "img/emotes/NoTears.47f758.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NoTears.8816d1.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NoTears.ce8ed2.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OverRustle",
    "versions": [
      {
        "path": "img/emotes/OverRustle.339f2e.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OverRustle.4638af.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/OverRustle.c88758.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DuckerZ",
    "versions": [
      {
        "path": "img/emotes/DuckerZ.bb488a.png",
        "animated": false,
        "dimensions": { "height": 26, "width": 56 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DuckerZ.2216fb.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 75 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DuckerZ.4cd156.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 149 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Kappa",
    "versions": [
      {
        "path": "img/emotes/Kappa.a63fd1.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 24 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Kappa.a00cb5.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 47 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Kappa.0c886b.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 94 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Klappa",
    "versions": [
      {
        "path": "img/emotes/Klappa.6d704d.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Klappa.bd8a54.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 66 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Klappa.a3bed2.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 132 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DappaKappa",
    "versions": [
      {
        "path": "img/emotes/DappaKappa.4843eb.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 26 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DappaKappa.23c945.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 52 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DappaKappa.cdffdb.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 104 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BibleThump",
    "versions": [
      {
        "path": "img/emotes/BibleThump.a5061c.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BibleThump.d3bf26.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 72 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BibleThump.4fc96e.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 144 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "AngelThump",
    "versions": [
      {
        "path": "img/emotes/AngelThump.a4ae27.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 95 },
        "size": "1x",
      },
      {
        "path": "img/emotes/AngelThump.14d062.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 167 },
        "size": "2x",
      },
      {
        "path": "img/emotes/AngelThump.e83822.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 334 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BasedGod",
    "versions": [
      {
        "path": "img/emotes/BasedGod.e36d5e.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BasedGod.f709c5.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BasedGod.40320c.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SoDoge",
    "versions": [
      {
        "path": "img/emotes/SoDoge.d8fa40.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 47 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SoDoge.16227b.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 94 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SoDoge.79146f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 189 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "WhoahDude",
    "versions": [
      {
        "path": "img/emotes/WhoahDude.de5c61.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 24 },
        "size": "1x",
      },
      {
        "path": "img/emotes/WhoahDude.3de11d.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 48 },
        "size": "2x",
      },
      {
        "path": "img/emotes/WhoahDude.d12803.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 97 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "MotherFuckinGame",
    "versions": [
      {
        "path": "img/emotes/MotherFuckinGame.7f5a5e.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MotherFuckinGame.d91f30.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MotherFuckinGame.795367.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 127 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DaFeels",
    "versions": [
      {
        "path": "img/emotes/DaFeels.c6a1a9.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DaFeels.8b99ae.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 113 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "UWOTM8",
    "versions": [
      {
        "path": "img/emotes/UWOTM8.f999e7.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/UWOTM8.ad3acc.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/UWOTM8.8f2374.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DatGeoff",
    "versions": [
      {
        "path": "img/emotes/DatGeoff.f24141.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 29 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DatGeoff.36ad41.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 57 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DatGeoff.754670.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 114 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FerretLOL",
    "versions": [
      {
        "path": "img/emotes/FerretLOL.324489.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FerretLOL.6dbe09.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 66 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FerretLOL.46bccd.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 132 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Sippy",
    "versions": [
      {
        "path": "img/emotes/Sippy.5b874a.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 29 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Sippy.783059.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 58 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Sippy.f8ae95.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 116 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Nappa",
    "versions": [
      {
        "path": "img/emotes/Nappa.d36033.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 22 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "DAFUK",
    "versions": [
      {
        "path": "img/emotes/DAFUK.8b1177.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DAFUK.9af784.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DAFUK.908cac.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "HEADSHOT",
    "versions": [
      {
        "path": "img/emotes/HEADSHOT.11ccfb.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 100 },
        "size": "1x",
      },
      {
        "path": "img/emotes/HEADSHOT.c85929.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 200 },
        "size": "2x",
      },
      {
        "path": "img/emotes/HEADSHOT.418ee7.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 400 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DANKMEMES",
    "versions": [
      {
        "path": "img/emotes/DANKMEMES.36e8ba.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 72 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DANKMEMES.e64837.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 144 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DANKMEMES.a79652.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 288 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "MLADY",
    "versions": [
      {
        "path": "img/emotes/MLADY.31d8f7.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MLADY.e57413.png",
        "animated": false,
        "dimensions": { "height": 55, "width": 52 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MLADY.dd3b9a.png",
        "animated": false,
        "dimensions": { "height": 109, "width": 104 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "MASTERB8",
    "versions": [
      {
        "path": "img/emotes/MASTERB8.dfd4b8.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 44 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MASTERB8.bc9039.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 87 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MASTERB8.e44fdf.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 175 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NOTMYTEMPO",
    "versions": [
      {
        "path": "img/emotes/NOTMYTEMPO.a26025.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 23 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NOTMYTEMPO.b3a487.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 46 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NOTMYTEMPO.8e5d4f.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 92 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "LeRuse",
    "versions": [
      {
        "path": "img/emotes/LeRuse.a2ab3b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/LeRuse.7edd21.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 73 },
        "size": "2x",
      },
      {
        "path": "img/emotes/LeRuse.d7a38f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 146 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "YEE",
    "versions": [
      {
        "path": "img/emotes/YEE.32e97d.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 20 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "SWEATY",
    "versions": [
      {
        "path": "img/emotes/SWEATY.bc8ad2.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SWEATY.832c89.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 71 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SWEATY.2b00e3.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 141 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PEPE",
    "versions": [
      {
        "path": "img/emotes/PEPE.d00282.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "SpookerZ",
    "versions": [
      {
        "path": "img/emotes/SpookerZ.b70a0b.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 34 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SpookerZ.216331.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 68 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SpookerZ.4a18b8.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 136 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "WEEWOO",
    "versions": [
      {
        "path": "img/emotes/WEEWOO.0a1c32.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/WEEWOO.75eb65.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 66 },
        "size": "2x",
      },
      {
        "path": "img/emotes/WEEWOO.0cb832.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 132 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ASLAN",
    "versions": [
      {
        "path": "img/emotes/ASLAN.016072.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 41 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ASLAN.dfe59d.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 82 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ASLAN.319499.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 164 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "TRUMPED",
    "versions": [
      {
        "path": "img/emotes/TRUMPED.09a9c0.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 49 },
        "size": "1x",
      },
      {
        "path": "img/emotes/TRUMPED.8aad4a.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 98 },
        "size": "2x",
      },
      {
        "path": "img/emotes/TRUMPED.e55272.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 196 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BASEDWATM8",
    "versions": [
      {
        "path": "img/emotes/BASEDWATM8.cb5eda.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 38 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BASEDWATM8.323a67.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 110 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BERN",
    "versions": [
      {
        "path": "img/emotes/BERN.e59124.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 45 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BERN.2d3ec5.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 90 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BERN.0b1158.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 180 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Hmmm",
    "versions": [
      {
        "path": "img/emotes/Hmmm.53a819.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Hmmm.b769f0.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Hmmm.efc59f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoThink",
    "versions": [
      {
        "path": "img/emotes/PepoThink.a87a40.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoThink.d5a70d.png",
        "animated": false,
        "dimensions": { "height": 61, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoThink.9c435c.png",
        "animated": false,
        "dimensions": { "height": 122, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsAmazingMan",
    "versions": [
      {
        "path": "img/emotes/FeelsAmazingMan.2f545b.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsAmazingMan.fc7399.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsAmazingMan.5c4999.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsBadMan",
    "versions": [
      {
        "path": "img/emotes/FeelsBadMan.0cebe6.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsBadMan.3868a7.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsBadMan.06b4fa.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsGoodMan",
    "versions": [
      {
        "path": "img/emotes/FeelsGoodMan.247be1.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsGoodMan.d23a31.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsGoodMan.8f4e65.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Woof",
    "versions": [
      {
        "path": "img/emotes/Woof.679a33.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Woof.d1df48.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Woof.a0d29a.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Wowee",
    "versions": [
      {
        "path": "img/emotes/Wowee.1cf6c2.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Wowee.2c2443.png",
        "animated": false,
        "dimensions": { "height": 54, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Wowee.2ff3d4.png",
        "animated": false,
        "dimensions": { "height": 108, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "haHAA",
    "versions": [
      {
        "path": "img/emotes/haHAA.3decf8.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/haHAA.37448f.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/haHAA.82fdae.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "POTATO",
    "versions": [
      {
        "path": "img/emotes/POTATO.c298a0.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/POTATO.b91f12.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/POTATO.d38135.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NOBULLY",
    "versions": [
      {
        "path": "img/emotes/NOBULLY.807d64.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 31 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NOBULLY.b91b9a.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 62 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NOBULLY.022a72.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 125 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "gachiGASM",
    "versions": [
      {
        "path": "img/emotes/gachiGASM.bd0956.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/gachiGASM.dc5d05.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 49 },
        "size": "2x",
      },
      {
        "path": "img/emotes/gachiGASM.3556e3.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 97 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "REE",
    "versions": [
      {
        "path": "img/emotes/REE.528417.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/REE.c67b5c.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/REE.c87c5c.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "monkaS",
    "versions": [
      {
        "path": "img/emotes/monkaS.020ab4.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/monkaS.ed4074.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/monkaS.b256be.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "RaveDoge",
    "versions": [
      {
        "path": "img/emotes/RaveDoge.0a5b8c.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 44 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "CuckCrab",
    "versions": [
      {
        "path": "img/emotes/CuckCrab.b3e670.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "MiyanoHype",
    "versions": [
      {
        "path": "img/emotes/MiyanoHype.7d624b.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "ECH",
    "versions": [
      {
        "path": "img/emotes/ECH.b305a5.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ECH.db10b4.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 61 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ECH.c82d64.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 121 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NiceMeMe",
    "versions": [
      {
        "path": "img/emotes/NiceMeMe.74882e.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 27 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NiceMeMe.f6377e.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 43 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NiceMeMe.53679c.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 86 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ITSRAWWW",
    "versions": [
      {
        "path": "img/emotes/ITSRAWWW.8b5d43.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 29 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ITSRAWWW.31fd89.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 58 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ITSRAWWW.55b9c2.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 116 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Riperino",
    "versions": [
      {
        "path": "img/emotes/Riperino.3438de.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Riperino.3aa208.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "4Head",
    "versions": [
      {
        "path": "img/emotes/4Head.d316d1.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 20 },
        "size": "1x",
      },
      {
        "path": "img/emotes/4Head.d010eb.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 41 },
        "size": "2x",
      },
      {
        "path": "img/emotes/4Head.aff6dd.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 81 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BabyRage",
    "versions": [
      {
        "path": "img/emotes/BabyRage.932831.png",
        "animated": false,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BabyRage.7bcd3f.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BabyRage.da8b4a.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Kreygasm",
    "versions": [
      {
        "path": "img/emotes/Kreygasm.be5029.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 25 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Kreygasm.229957.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 50 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Kreygasm.bc46f4.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 99 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SMOrc",
    "versions": [
      {
        "path": "img/emotes/SMOrc.543efb.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SMOrc.1c3906.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 62 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SMOrc.115a85.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 123 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NotLikeThis",
    "versions": [
      {
        "path": "img/emotes/NotLikeThis.e434c7.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NotLikeThis.e898d0.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NotLikeThis.e67520.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "POGGERS",
    "versions": [
      {
        "path": "img/emotes/POGGERS.298a5a.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/POGGERS.334311.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/POGGERS.2b1135.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "AYAYA",
    "versions": [
      {
        "path": "img/emotes/AYAYA.e1da92.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 41 },
        "size": "1x",
      },
      {
        "path": "img/emotes/AYAYA.94e4f2.png",
        "animated": false,
        "dimensions": { "height": 55, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/AYAYA.197db9.png",
        "animated": false,
        "dimensions": { "height": 109, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepOk",
    "versions": [
      {
        "path": "img/emotes/PepOk.15fbde.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepOk.e37ab7.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 73 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepOk.c9c93f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 146 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoComfy",
    "versions": [
      {
        "path": "img/emotes/PepoComfy.4455dd.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoComfy.03eeaf.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoComfy.6fc0d0.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoWant",
    "versions": [
      {
        "path": "img/emotes/PepoWant.af9e41.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 38 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoWant.dd12bc.png",
        "animated": false,
        "dimensions": { "height": 44, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoWant.9eb0a2.png",
        "animated": false,
        "dimensions": { "height": 88, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepeHands",
    "versions": [
      {
        "path": "img/emotes/PepeHands.290f43.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepeHands.3affbf.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepeHands.511ba2.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BOGGED",
    "versions": [
      {
        "path": "img/emotes/BOGGED.d47e7f.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BOGGED.19c208.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BOGGED.d42125.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyApe",
    "versions": [
      {
        "path": "img/emotes/ComfyApe.8888d0.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyApe.8888d0.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ApeHands",
    "versions": [
      {
        "path": "img/emotes/ApeHands.2309ab.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ApeHands.2a521f.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ApeHands.951e7d.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OMEGALUL",
    "versions": [
      {
        "path": "img/emotes/OMEGALUL.9209ba.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 31 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OMEGALUL.18e4c3.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "COGGERS",
    "versions": [
      {
        "path": "img/emotes/COGGERS.3acb2e.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "1x",
      },
      {
        "path": "img/emotes/COGGERS.3acb2e.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "2x",
      },
      {
        "path": "img/emotes/COGGERS.3acb2e.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoWant",
    "versions": [
      {
        "path": "img/emotes/PepoWant.af9e41.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 38 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoWant.dd12bc.png",
        "animated": false,
        "dimensions": { "height": 44, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoWant.9eb0a2.png",
        "animated": false,
        "dimensions": { "height": 88, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Clap",
    "versions": [
      {
        "path": "img/emotes/Clap.fd04d4.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 22 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "FeelsWeirdMan",
    "versions": [
      {
        "path": "img/emotes/FeelsWeirdMan.7c55eb.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsWeirdMan.301ee9.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsWeirdMan.fadd68.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "monkaMEGA",
    "versions": [
      {
        "path": "img/emotes/monkaMEGA.2ad1c9.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/monkaMEGA.b4b7d0.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/monkaMEGA.9f0728.png",
        "animated": false,
        "dimensions": { "height": 111, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyDog",
    "versions": [
      {
        "path": "img/emotes/ComfyDog.9be7ff.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyDog.fdd582.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyDog.4b3e84.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "GIMI",
    "versions": [
      {
        "path": "img/emotes/GIMI.a689d3.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/GIMI.eca13d.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 72 },
        "size": "2x",
      },
      {
        "path": "img/emotes/GIMI.eb3b29.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 144 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "MOOBERS",
    "versions": [
      {
        "path": "img/emotes/MOOBERS.11a2aa.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MOOBERS.e57c04.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MOOBERS.7bb82b.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoBan",
    "versions": [
      {
        "path": "img/emotes/PepoBan.a227bb.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 40 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoBan.a8bf13.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 80 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoBan.7da749.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 160 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyAYA",
    "versions": [
      {
        "path": "img/emotes/ComfyAYA.88d999.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyAYA.110e1e.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyAYA.854db8.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyFerret",
    "versions": [
      {
        "path": "img/emotes/ComfyFerret.1b9ddd.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 34 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyFerret.052ea6.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 68 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyFerret.bb67ae.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 136 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "BOOMER",
    "versions": [
      {
        "path": "img/emotes/BOOMER.95255e.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/BOOMER.69de24.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/BOOMER.f0fdb3.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ZOOMER",
    "versions": [
      {
        "path": "img/emotes/ZOOMER.551103.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ZOOMER.1fd5c9.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ZOOMER.213fbc.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SOY",
    "versions": [
      {
        "path": "img/emotes/SOY.98c41b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 34 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SOY.d7eb8b.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 68 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SOY.7416f4.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 137 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsPepoMan",
    "versions": [
      {
        "path": "img/emotes/FeelsPepoMan.e517c9.gif",
        "animated": true,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsPepoMan.c1ebf1.gif",
        "animated": true,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsPepoMan.8b8ed5.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyCat",
    "versions": [
      {
        "path": "img/emotes/ComfyCat.315507.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyCat.13c1f7.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyCat.7b8c23.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyPOTATO",
    "versions": [
      {
        "path": "img/emotes/ComfyPOTATO.c659d4.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyPOTATO.21dde4.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyPOTATO.218ac6.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SUGOI",
    "versions": [
      {
        "path": "img/emotes/SUGOI.71ef4c.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SUGOI.4a65f9.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SUGOI.fafbe2.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DJPepo",
    "versions": [
      {
        "path": "img/emotes/DJPepo.11df32.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DJPepo.8eff19.png",
        "animated": false,
        "dimensions": { "height": 46, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DJPepo.0a3fa4.png",
        "animated": false,
        "dimensions": { "height": 91, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "CampFire",
    "versions": [
      {
        "path": "img/emotes/CampFire.ee8cc3.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/CampFire.f6add5.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/CampFire.16713e.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyYEE",
    "versions": [
      {
        "path": "img/emotes/ComfyYEE.1db778.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "weSmart",
    "versions": [
      {
        "path": "img/emotes/weSmart.1582be.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/weSmart.8ae56a.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 55 },
        "size": "2x",
      },
      {
        "path": "img/emotes/weSmart.a5ab35.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 109 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoG",
    "versions": [
      {
        "path": "img/emotes/PepoG.3979b9.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoG.b4eb7e.png",
        "animated": false,
        "dimensions": { "height": 52, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoG.16975c.png",
        "animated": false,
        "dimensions": { "height": 103, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OBJECTION",
    "versions": [
      {
        "path": "img/emotes/OBJECTION.d92a4c.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OBJECTION.fec728.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 71 },
        "size": "2x",
      },
      {
        "path": "img/emotes/OBJECTION.1895b3.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 141 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyWeird",
    "versions": [
      {
        "path": "img/emotes/ComfyWeird.33bfda.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ComfyWeird.20ee4b.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ComfyWeird.e5eb6a.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "umaruCry",
    "versions": [
      {
        "path": "img/emotes/umaruCry.5e93ef.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/umaruCry.60d64a.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/umaruCry.4e35ba.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OsKrappa",
    "versions": [
      {
        "path": "img/emotes/OsKrappa.7869f7.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OsKrappa.126161.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 72 },
        "size": "2x",
      },
      {
        "path": "img/emotes/OsKrappa.ece214.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 144 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "monkaHmm",
    "versions": [
      {
        "path": "img/emotes/monkaHmm.ac01b6.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/monkaHmm.2d738c.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/monkaHmm.21fc0d.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoHmm",
    "versions": [
      {
        "path": "img/emotes/PepoHmm.1a4fd2.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoHmm.42925f.png",
        "animated": false,
        "dimensions": { "height": 45, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoHmm.6223cd.png",
        "animated": false,
        "dimensions": { "height": 89, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepeComfy",
    "versions": [
      {
        "path": "img/emotes/PepeComfy.0fb740.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 34 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepeComfy.d86782.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepeComfy.09b795.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SUGOwO",
    "versions": [
      {
        "path": "img/emotes/SUGOwO.78ed2f.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SUGOwO.8edb48.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SUGOwO.68a46f.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "EZ",
    "versions": [
      {
        "path": "img/emotes/EZ.79706e.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 34 },
        "size": "1x",
      },
      {
        "path": "img/emotes/EZ.a1d922.png",
        "animated": false,
        "dimensions": { "height": 50, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/EZ.ecff51.png",
        "animated": false,
        "dimensions": { "height": 100, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Pepega",
    "versions": [
      {
        "path": "img/emotes/Pepega.7470b2.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Pepega.efc213.png",
        "animated": false,
        "dimensions": { "height": 49, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Pepega.9acdf3.png",
        "animated": false,
        "dimensions": { "height": 98, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "shyLurk",
    "versions": [
      {
        "path": "img/emotes/shyLurk.8bbb56.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/shyLurk.ce2ced.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/shyLurk.786937.png",
        "animated": false,
        "dimensions": { "height": 63, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsOkayMan",
    "versions": [
      {
        "path": "img/emotes/FeelsOkayMan.782627.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsOkayMan.a2202d.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
    ],
  },
  {
    "name": "POKE",
    "versions": [
      {
        "path": "img/emotes/POKE.eb6024.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/POKE.3baef6.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/POKE.86922f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoDance",
    "versions": [
      {
        "path": "img/emotes/PepoDance.038b22.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "ORDAH",
    "versions": [
      {
        "path": "img/emotes/ORDAH.f66e06.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/ORDAH.38f716.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/ORDAH.e7af40.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SPY",
    "versions": [
      {
        "path": "img/emotes/SPY.2e2c8a.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SPY.f53156.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 69 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SPY.5493c6.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 138 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoGood",
    "versions": [
      {
        "path": "img/emotes/PepoGood.720d40.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 52 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoGood.849d93.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 103 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoGood.21564a.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 206 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepeJam",
    "versions": [
      {
        "path": "img/emotes/PepeJam.24ca04.gif",
        "animated": true,
        "dimensions": { "height": 108, "width": 112 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "LAG",
    "versions": [
      {
        "path": "img/emotes/LAG.50aef6.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 40 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "billyWeird",
    "versions": [
      {
        "path": "img/emotes/billyWeird.f0f0de.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/billyWeird.74f694.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/billyWeird.902912.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SOTRIGGERED",
    "versions": [
      {
        "path": "img/emotes/SOTRIGGERED.1ba48b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SOTRIGGERED.d34779.png",
        "animated": false,
        "dimensions": { "height": 59, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SOTRIGGERED.dd0913.png",
        "animated": false,
        "dimensions": { "height": 119, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OnlyPretending",
    "versions": [
      {
        "path": "img/emotes/OnlyPretending.275103.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OnlyPretending.00e027.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/OnlyPretending.4f6754.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "cmonBruh",
    "versions": [
      {
        "path": "img/emotes/cmonBruh.655e42.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/cmonBruh.959e96.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 61 },
        "size": "2x",
      },
      {
        "path": "img/emotes/cmonBruh.21f844.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 122 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "VroomVroom",
    "versions": [
      {
        "path": "img/emotes/VroomVroom.33908b.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "mikuDance",
    "versions": [
      {
        "path": "img/emotes/mikuDance.460a4d.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 28 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "WAG",
    "versions": [
      {
        "path": "img/emotes/WAG.f53f6a.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 37 },
        "size": "1x",
      },
      {
        "path": "img/emotes/WAG.19e494.gif",
        "animated": true,
        "dimensions": { "height": 64, "width": 74 },
        "size": "2x",
      },
      {
        "path": "img/emotes/WAG.a8b3d2.gif",
        "animated": true,
        "dimensions": { "height": 128, "width": 148 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoFight",
    "versions": [
      {
        "path": "img/emotes/PepoFight.ac5760.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "PepeLaugh",
    "versions": [
      {
        "path": "img/emotes/PepeLaugh.61be17.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepeLaugh.860dbf.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepeLaugh.46139f.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PeepoS",
    "versions": [
      {
        "path": "img/emotes/PeepoS.e4cf2a.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PeepoS.9a3b37.png",
        "animated": false,
        "dimensions": { "height": 40, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PeepoS.084892.png",
        "animated": false,
        "dimensions": { "height": 79, "width": 111 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SLEEPY",
    "versions": [
      {
        "path": "img/emotes/SLEEPY.ca4cd1.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 58 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SLEEPY.9edd65.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 116 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SLEEPY.eaed40.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 231 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "GODMAN",
    "versions": [
      {
        "path": "img/emotes/GODMAN.c7d698.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/GODMAN.2e1199.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 72 },
        "size": "2x",
      },
      {
        "path": "img/emotes/GODMAN.9f3513.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 144 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "NOM",
    "versions": [
      {
        "path": "img/emotes/NOM.340918.png",
        "animated": false,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/NOM.51b2fe.png",
        "animated": false,
        "dimensions": { "height": 60, "width": 60 },
        "size": "2x",
      },
      {
        "path": "img/emotes/NOM.d51140.png",
        "animated": false,
        "dimensions": { "height": 120, "width": 120 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsDumbMan",
    "versions": [
      {
        "path": "img/emotes/FeelsDumbMan.3d7552.png",
        "animated": false,
        "dimensions": { "height": 26, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/FeelsDumbMan.33be80.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/FeelsDumbMan.a25e7c.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SEMPAI",
    "versions": [
      {
        "path": "img/emotes/SEMPAI.1357e4.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "OSTRIGGERED",
    "versions": [
      {
        "path": "img/emotes/OSTRIGGERED.ba2953.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 52 },
        "size": "1x",
      },
      {
        "path": "img/emotes/OSTRIGGERED.b15d18.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 103 },
        "size": "2x",
      },
      {
        "path": "img/emotes/OSTRIGGERED.4c921e.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 206 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "MiyanoBird",
    "versions": [
      {
        "path": "img/emotes/MiyanoBird.896c6b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MiyanoBird.960657.png",
        "animated": false,
        "dimensions": { "height": 54, "width": 53 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MiyanoBird.373dc1.png",
        "animated": false,
        "dimensions": { "height": 107, "width": 106 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "KING",
    "versions": [
      {
        "path": "img/emotes/KING.ef9df4.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "PIKOHH",
    "versions": [
      {
        "path": "img/emotes/PIKOHH.845754.png",
        "animated": false,
        "dimensions": { "height": 25, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PIKOHH.a8014d.png",
        "animated": false,
        "dimensions": { "height": 50, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PIKOHH.02d8a7.png",
        "animated": false,
        "dimensions": { "height": 100, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoPirate",
    "versions": [
      {
        "path": "img/emotes/PepoPirate.dff9ba.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoPirate.929d0e.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 71 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoPirate.c0d79d.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 142 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepeMods",
    "versions": [
      {
        "path": "img/emotes/PepeMods.ec1cef.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepeMods.4b7155.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepeMods.92f328.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "OhISee",
    "versions": [
      {
        "path": "img/emotes/OhISee.7b2c9f.gif",
        "animated": true,
        "dimensions": { "height": 125, "width": 128 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "WeirdChamp",
    "versions": [
      {
        "path": "img/emotes/WeirdChamp.2e22c2.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/WeirdChamp.bed132.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/WeirdChamp.7208f9.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "RedCard",
    "versions": [
      {
        "path": "img/emotes/RedCard.e66eed.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 22 },
        "size": "1x",
      },
      {
        "path": "img/emotes/RedCard.e12101.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 44 },
        "size": "2x",
      },
      {
        "path": "img/emotes/RedCard.1e6105.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 88 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "illyaTriggered",
    "versions": [
      {
        "path": "img/emotes/illyaTriggered.9d2229.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "SadBenis",
    "versions": [
      {
        "path": "img/emotes/SadBenis.24ff60.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "PeepoHappy",
    "versions": [
      {
        "path": "img/emotes/PeepoHappy.d6cc0b.png",
        "animated": false,
        "dimensions": { "height": 20, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PeepoHappy.2cbd28.png",
        "animated": false,
        "dimensions": { "height": 40, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PeepoHappy.cec544.png",
        "animated": false,
        "dimensions": { "height": 79, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "ComfyWAG",
    "versions": [
      {
        "path": "img/emotes/ComfyWAG.dfd582.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 38 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "MiyanoComfy",
    "versions": [
      {
        "path": "img/emotes/MiyanoComfy.48abf8.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/MiyanoComfy.e68f92.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/MiyanoComfy.11d696.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "sataniaLUL",
    "versions": [
      {
        "path": "img/emotes/sataniaLUL.5b020b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/sataniaLUL.de547a.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/sataniaLUL.7fa030.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DELUSIONAL",
    "versions": [
      {
        "path": "img/emotes/DELUSIONAL.b63d89.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DELUSIONAL.322b47.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 66 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DELUSIONAL.171738.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 131 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "GREED",
    "versions": [
      {
        "path": "img/emotes/GREED.546433.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 30 },
        "size": "1x",
      },
      {
        "path": "img/emotes/GREED.ff6aef.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 61 },
        "size": "2x",
      },
      {
        "path": "img/emotes/GREED.66725a.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 121 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "AYAWeird",
    "versions": [
      {
        "path": "img/emotes/AYAWeird.b9b4fa.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/AYAWeird.af96d8.png",
        "animated": false,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/AYAWeird.cc1563.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "FeelsCountryMan",
    "versions": [
      {
        "path": "img/emotes/FeelsCountryMan.c3de30.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "SNAP",
    "versions": [
      {
        "path": "img/emotes/SNAP.2c0e6e.gif",
        "animated": true,
        "dimensions": { "height": 128, "width": 128 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "PeepoRiot",
    "versions": [
      {
        "path": "img/emotes/PeepoRiot.c79cde.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 36 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PeepoRiot.5ee52b.png",
        "animated": false,
        "dimensions": { "height": 43, "width": 62 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PeepoRiot.fee583.png",
        "animated": false,
        "dimensions": { "height": 86, "width": 124 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "HiHi",
    "versions": [
      {
        "path": "img/emotes/HiHi.477184.gif",
        "animated": true,
        "dimensions": { "height": 30, "width": 30 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "ComfyFeels",
    "versions": [
      {
        "path": "img/emotes/ComfyFeels.7182bd.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "MiyanoSip",
    "versions": [
      {
        "path": "img/emotes/MiyanoSip.ed4755.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "PeepoWeird",
    "versions": [
      {
        "path": "img/emotes/PeepoWeird.a05619.png",
        "animated": false,
        "dimensions": { "height": 20, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PeepoWeird.36e6d5.png",
        "animated": false,
        "dimensions": { "height": 40, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PeepoWeird.065ece.png",
        "animated": false,
        "dimensions": { "height": 79, "width": 111 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "JimFace",
    "versions": [
      {
        "path": "img/emotes/JimFace.b7be92.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 31 },
        "size": "1x",
      },
      {
        "path": "img/emotes/JimFace.dde361.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 62 },
        "size": "2x",
      },
      {
        "path": "img/emotes/JimFace.d26a2c.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 124 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "HACKER",
    "versions": [
      {
        "path": "img/emotes/HACKER.49cb28.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 48 },
        "size": "1x",
      },
      {
        "path": "img/emotes/HACKER.657379.gif",
        "animated": true,
        "dimensions": { "height": 64, "width": 101 },
        "size": "2x",
      },
      {
        "path": "img/emotes/HACKER.7f0ea4.gif",
        "animated": true,
        "dimensions": { "height": 128, "width": 202 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "monkaVirus",
    "versions": [
      {
        "path": "img/emotes/monkaVirus.e8dd2f.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/monkaVirus.f9754d.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/monkaVirus.a96ff5.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DOUBT",
    "versions": [
      {
        "path": "img/emotes/DOUBT.4869d8.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DOUBT.122405.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DOUBT.630f50.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "KEKW",
    "versions": [
      {
        "path": "img/emotes/KEKW.dbc0b9.gif",
        "animated": true,
        "dimensions": { "height": 32, "width": 64 },
        "size": "1x",
      },
    ],
  },
  {
    "name": "SHOCK",
    "versions": [
      {
        "path": "img/emotes/SHOCK.cfcdda.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 33 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SHOCK.fc5439.png",
        "animated": false,
        "dimensions": { "height": 761, "width": 790 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SHOCK.c795a4.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 133 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "DOIT",
    "versions": [
      {
        "path": "img/emotes/DOIT.cbc7b5.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 46 },
        "size": "1x",
      },
      {
        "path": "img/emotes/DOIT.267215.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 92 },
        "size": "2x",
      },
      {
        "path": "img/emotes/DOIT.ddbce0.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 183 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "GODWOMAN",
    "versions": [
      {
        "path": "img/emotes/GODWOMAN.29e449.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 53 },
        "size": "1x",
      },
      {
        "path": "img/emotes/GODWOMAN.a646a6.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 107 },
        "size": "2x",
      },
      {
        "path": "img/emotes/GODWOMAN.118116.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 213 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "POGGIES",
    "versions": [
      {
        "path": "img/emotes/POGGIES.d14d5d.png",
        "animated": false,
        "dimensions": { "height": 27, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/POGGIES.ddc555.png",
        "animated": false,
        "dimensions": { "height": 53, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/POGGIES.324fb1.png",
        "animated": false,
        "dimensions": { "height": 106, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "SHRUG",
    "versions": [
      {
        "path": "img/emotes/SHRUG.e024cd.png",
        "animated": false,
        "dimensions": { "height": 29, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/SHRUG.fb9779.png",
        "animated": false,
        "dimensions": { "height": 58, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/SHRUG.754503.png",
        "animated": false,
        "dimensions": { "height": 115, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "POGOI",
    "versions": [
      {
        "path": "img/emotes/POGOI.3fda56.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 31 },
        "size": "1x",
      },
      {
        "path": "img/emotes/POGOI.042acc.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 63 },
        "size": "2x",
      },
      {
        "path": "img/emotes/POGOI.597ba9.png",
        "animated": false,
        "dimensions": { "height": 112, "width": 110 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "PepoSleep",
    "versions": [
      {
        "path": "img/emotes/PepoSleep.8a0d23.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 35 },
        "size": "1x",
      },
      {
        "path": "img/emotes/PepoSleep.6270f1.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 70 },
        "size": "2x",
      },
      {
        "path": "img/emotes/PepoSleep.dd0a79.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 141 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "KEKE",
    "versions": [
      {
        "path": "img/emotes/KEKE.fdb512.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/KEKE.d2f0e0.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/KEKE.b9f85b.png",
        "animated": false,
        "dimensions": { "height": 128, "width": 128 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "catJAM",
    "versions": [
      {
        "path": "img/emotes/catJAM.41eb16.gif",
        "animated": true,
        "dimensions": { "height": 28, "width": 28 },
        "size": "1x",
      },
      {
        "path": "img/emotes/catJAM.855d21.gif",
        "animated": true,
        "dimensions": { "height": 56, "width": 56 },
        "size": "2x",
      },
      {
        "path": "img/emotes/catJAM.8245fa.gif",
        "animated": true,
        "dimensions": { "height": 112, "width": 112 },
        "size": "4x",
      },
    ],
  },
  {
    "name": "Facepalm",
    "versions": [
      {
        "path": "img/emotes/Facepalm.66269b.png",
        "animated": false,
        "dimensions": { "height": 32, "width": 32 },
        "size": "1x",
      },
      {
        "path": "img/emotes/Facepalm.096bf9.png",
        "animated": false,
        "dimensions": { "height": 64, "width": 64 },
        "size": "2x",
      },
      {
        "path": "img/emotes/Facepalm.d29040.png",
        "animated": false,
        "dimensions": { "height": 127, "width": 128 },
        "size": "4x",
      },
    ],
  },
];

emotes.forEach((emote) =>
  emote.versions.forEach((version) => (version.url = `https://chat.strims.gg/${version.path}`))
);

export const modifiers = [
  "fast",
  "flip",
  "hyper",
  "lag",
  "love",
  "mirror",
  "rain",
  "rustle",
  "slow",
  "snow",
  "spin",
  "wide",
  "worth",
];
