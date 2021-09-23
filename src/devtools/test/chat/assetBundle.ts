import * as chatv1 from "../../../apis/strims/chat/v1/chat";
import imgBONK from "./emotes/animated/BONK.png";
import imgcatJAM from "./emotes/animated/catJAM.png";
import imgCinnabunny from "./emotes/animated/Cinnabunny.png";
import imgComfyMoobers from "./emotes/animated/ComfyMoobers.png";
import imgDuckJAM from "./emotes/animated/DuckJAM.png";
import imgNODDERS from "./emotes/animated/NODDERS.png";
import imgNOPERS from "./emotes/animated/NOPERS.png";
import imgPeepoRun from "./emotes/animated/PeepoRun.png";
import imgREE from "./emotes/animated/REE.png";
import imgRIDIN from "./emotes/animated/RIDIN.png";
import imgTANTIES from "./emotes/animated/TANTIES.png";
import imgVroomVroom from "./emotes/animated/VroomVroom.png";
import imgWAG from "./emotes/animated/WAG.png";
import imgWAYTOODANK from "./emotes/animated/WAYTOODANK.png";
import img4Head from "./emotes/static/4Head.png";
import img4U from "./emotes/static/4U.png";
import imgAbathur from "./emotes/static/Abathur.png";
import imgAngelThump from "./emotes/static/AngelThump.png";
import imgAOCFace from "./emotes/static/AOCFace.png";
import imgApeHands from "./emotes/static/ApeHands.png";
import imgASLAN from "./emotes/static/ASLAN.png";
import imgAYAWeird from "./emotes/static/AYAWeird.png";
import imgAYAYA from "./emotes/static/AYAYA.png";
import imgBabyRage from "./emotes/static/BabyRage.png";
import imgBasedGod from "./emotes/static/BasedGod.png";
import imgBASEDWATM8 from "./emotes/static/BASEDWATM8.png";
import imgBERN from "./emotes/static/BERN.png";
import imgBibleThump from "./emotes/static/BibleThump.png";
import imgbillyWeird from "./emotes/static/billyWeird.png";
import imgBOGGED from "./emotes/static/BOGGED.png";
import imgBOOMER from "./emotes/static/BOOMER.png";
import imgCampFire from "./emotes/static/CampFire.png";
// import imgCinnabunny from "./emotes/static/Cinnabunny.png";
import imgcmonBruh from "./emotes/static/cmonBruh.png";
import imgComfyApe from "./emotes/static/ComfyApe.png";
import imgComfyAYA from "./emotes/static/ComfyAYA.png";
import imgComfyCat from "./emotes/static/ComfyCat.png";
import imgComfyDog from "./emotes/static/ComfyDog.png";
import imgComfyFeels from "./emotes/static/ComfyFeels.png";
import imgComfyFerret from "./emotes/static/ComfyFerret.png";
import imgComfyPOTATO from "./emotes/static/ComfyPOTATO.png";
import imgComfyWeird from "./emotes/static/ComfyWeird.png";
import imgCOPIUM from "./emotes/static/COPIUM.png";
import imgDaFeels from "./emotes/static/DaFeels.png";
import imgDAFUK from "./emotes/static/DAFUK.png";
import imgDANKMEMES from "./emotes/static/DANKMEMES.png";
import imgDappaKappa from "./emotes/static/DappaKappa.png";
import imgDatGeoff from "./emotes/static/DatGeoff.png";
import imgDELUSIONAL from "./emotes/static/DELUSIONAL.png";
import imgDJPepo from "./emotes/static/DJPepo.png";
import imgDOGGO from "./emotes/static/DOGGO.png";
import imgDOIT from "./emotes/static/DOIT.png";
import imgDOUBT from "./emotes/static/DOUBT.png";
import imgDuckerZ from "./emotes/static/DuckerZ.png";
import imgECH from "./emotes/static/ECH.png";
import imgEZ from "./emotes/static/EZ.png";
import imgFacepalm from "./emotes/static/Facepalm.png";
import imgFeelsAmazingMan from "./emotes/static/FeelsAmazingMan.png";
import imgFeelsBadMan from "./emotes/static/FeelsBadMan.png";
import imgFeelsDumbMan from "./emotes/static/FeelsDumbMan.png";
import imgFeelsGoodMan from "./emotes/static/FeelsGoodMan.png";
import imgFeelsOkayMan from "./emotes/static/FeelsOkayMan.png";
import imgFeelsStrongMan from "./emotes/static/FeelsStrongMan.png";
import imgFeelsWeirdMan from "./emotes/static/FeelsWeirdMan.png";
import imgFerretLOL from "./emotes/static/FerretLOL.png";
import imgFIDGETLOL from "./emotes/static/FIDGETLOL.png";
import imggachiGASM from "./emotes/static/gachiGASM.png";
import imgGameOfThrows from "./emotes/static/GameOfThrows.png";
import imgGandalfFace from "./emotes/static/GandalfFace.png";
import imgGIMI from "./emotes/static/GIMI.png";
import imgGODMAN from "./emotes/static/GODMAN.png";
import imgGODWOMAN from "./emotes/static/GODWOMAN.png";
import imgGREED from "./emotes/static/GREED.png";
import imghaHAA from "./emotes/static/haHAA.png";
import imgHEADSHOT from "./emotes/static/HEADSHOT.png";
import imgHhhehhehe from "./emotes/static/Hhhehhehe.png";
import imgHmmm from "./emotes/static/Hmmm.png";
import imgINFESTOR from "./emotes/static/INFESTOR.png";
import imgITSRAWWW from "./emotes/static/ITSRAWWW.png";
import imgJimFace from "./emotes/static/JimFace.png";
import imgKappa from "./emotes/static/Kappa.png";
import imgKlappa from "./emotes/static/Klappa.png";
import imgKreygasm from "./emotes/static/Kreygasm.png";
import imgLeRuse from "./emotes/static/LeRuse.png";
import imgLUL from "./emotes/static/LUL.png";
import imgMASTERB8 from "./emotes/static/MASTERB8.png";
import imgMiyanoBird from "./emotes/static/MiyanoBird.png";
import imgMiyanoComfy from "./emotes/static/MiyanoComfy.png";
import imgMLADY from "./emotes/static/MLADY.png";
import imgmonkaHmm from "./emotes/static/monkaHmm.png";
import imgmonkaMEGA from "./emotes/static/monkaMEGA.png";
import imgmonkaS from "./emotes/static/monkaS.png";
import imgmonkaVirus from "./emotes/static/monkaVirus.png";
import imgMOOBERS from "./emotes/static/MOOBERS.png";
import imgMotherFuckinGame from "./emotes/static/MotherFuckinGame.png";
import imgNeneLaugh from "./emotes/static/NeneLaugh.png";
import imgNiceMeMe from "./emotes/static/NiceMeMe.png";
import imgNOBULLY from "./emotes/static/NOBULLY.png";
import imgNOM from "./emotes/static/NOM.png";
import imgNoTears from "./emotes/static/NoTears.png";
import imgNotLikeThis from "./emotes/static/NotLikeThis.png";
import imgNOTMYTEMPO from "./emotes/static/NOTMYTEMPO.png";
import imgOBJECTION from "./emotes/static/OBJECTION.png";
import imgOHDEAR from "./emotes/static/OHDEAR.png";
import imgOMEGALUL from "./emotes/static/OMEGALUL.png";
import imgOnlyPretending from "./emotes/static/OnlyPretending.png";
import imgORDAH from "./emotes/static/ORDAH.png";
import imgOsKrappa from "./emotes/static/OsKrappa.png";
import imgOSTRIGGERED from "./emotes/static/OSTRIGGERED.png";
import imgOverRustle from "./emotes/static/OverRustle.png";
import imgPAIN from "./emotes/static/PAIN.png";
import imgPauseChamp from "./emotes/static/PauseChamp.png";
import imgPeepoHappy from "./emotes/static/PeepoHappy.png";
import imgPeepoRiot from "./emotes/static/PeepoRiot.png";
import imgPeepoS from "./emotes/static/PeepoS.png";
import imgpeepoWave from "./emotes/static/peepoWave.png";
import imgPeepoWeird from "./emotes/static/PeepoWeird.png";
import imgPEPE from "./emotes/static/PEPE.png";
import imgPepeComfy from "./emotes/static/PepeComfy.png";
import imgPepega from "./emotes/static/Pepega.png";
import imgPepeHands from "./emotes/static/PepeHands.png";
import imgPepeLaugh from "./emotes/static/PepeLaugh.png";
import imgPepeMods from "./emotes/static/PepeMods.png";
import imgPepoBan from "./emotes/static/PepoBan.png";
import imgPepoComfy from "./emotes/static/PepoComfy.png";
import imgPepoG from "./emotes/static/PepoG.png";
import imgPepoGood from "./emotes/static/PepoGood.png";
import imgPepoHmm from "./emotes/static/PepoHmm.png";
import imgPepOk from "./emotes/static/PepOk.png";
import imgPepoPirate from "./emotes/static/PepoPirate.png";
import imgPepoSleep from "./emotes/static/PepoSleep.png";
import imgPepoThink from "./emotes/static/PepoThink.png";
import imgPepoWant from "./emotes/static/PepoWant.png";
import imgPIKOHH from "./emotes/static/PIKOHH.png";
import imgPOGGERS from "./emotes/static/POGGERS.png";
import imgPOGGIES from "./emotes/static/POGGIES.png";
import imgPOGOI from "./emotes/static/POGOI.png";
import imgPOKE from "./emotes/static/POKE.png";
import imgPOTATO from "./emotes/static/POTATO.png";
import imgPOUT from "./emotes/static/POUT.png";
import imgQUEEN from "./emotes/static/QUEEN.png";
import imgRedCard from "./emotes/static/RedCard.png";
// import imgREE from "./emotes/static/REE.png";
import imgRiperino from "./emotes/static/Riperino.png";
import imgsataniaLUL from "./emotes/static/sataniaLUL.png";
import imgSEMPAI from "./emotes/static/SEMPAI.png";
import imgSHOCK from "./emotes/static/SHOCK.png";
import imgSHRUG from "./emotes/static/SHRUG.png";
import imgshyLurk from "./emotes/static/shyLurk.png";
import imgSICKOOH from "./emotes/static/SICKOOH.png";
import imgSippy from "./emotes/static/Sippy.png";
import imgSLEEPY from "./emotes/static/SLEEPY.png";
import imgSMOrc from "./emotes/static/SMOrc.png";
import imgSMUG from "./emotes/static/SMUG.png";
import imgSoDoge from "./emotes/static/SoDoge.png";
import imgSOTRIGGERED from "./emotes/static/SOTRIGGERED.png";
import imgSpookerZ from "./emotes/static/SpookerZ.png";
import imgSPY from "./emotes/static/SPY.png";
import imgSUGOI from "./emotes/static/SUGOI.png";
import imgSUGOwO from "./emotes/static/SUGOwO.png";
import imgSURPRISE from "./emotes/static/SURPRISE.png";
import imgSWEATY from "./emotes/static/SWEATY.png";
import imgTIMID from "./emotes/static/TIMID.png";
import imgumaruCry from "./emotes/static/umaruCry.png";
import imgUWOTM8 from "./emotes/static/UWOTM8.png";
import imgWEEWOO from "./emotes/static/WEEWOO.png";
import imgweSmart from "./emotes/static/weSmart.png";
import imgWhoahDude from "./emotes/static/WhoahDude.png";
import imgWICKED from "./emotes/static/WICKED.png";
import imgWoof from "./emotes/static/Woof.png";
import imgWowee from "./emotes/static/Wowee.png";
import imgYEE from "./emotes/static/YEE.png";
import imgYellowCard from "./emotes/static/YellowCard.png";
import imgZOOMER from "./emotes/static/ZOOMER.png";

const src = {
  "isDelta": false,
  "room": {
    "name": "test server",
    "effects": [],
  },
  "emotes": [
    {
      "id": "2401",
      "name": "4Head",
      "images": [
        {
          "src": img4Head,
          "fileType": 1,
          "height": 128,
          "width": 81,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2402",
      "name": "4U",
      "images": [
        {
          "src": img4U,
          "fileType": 1,
          "height": 128,
          "width": 97,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2403",
      "name": "Abathur",
      "images": [
        {
          "src": imgAbathur,
          "fileType": 1,
          "height": 128,
          "width": 336,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2404",
      "name": "AngelThump",
      "images": [
        {
          "src": imgAngelThump,
          "fileType": 1,
          "height": 112,
          "width": 334,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2405",
      "name": "AOCFace",
      "images": [
        {
          "src": imgAOCFace,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2406",
      "name": "ApeHands",
      "images": [
        {
          "src": imgApeHands,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2407",
      "name": "ASLAN",
      "images": [
        {
          "src": imgASLAN,
          "fileType": 1,
          "height": 120,
          "width": 164,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2408",
      "name": "AYAWeird",
      "images": [
        {
          "src": imgAYAWeird,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2409",
      "name": "AYAYA",
      "images": [
        {
          "src": imgAYAYA,
          "fileType": 1,
          "height": 109,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2410",
      "name": "BabyRage",
      "images": [
        {
          "src": imgBabyRage,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2411",
      "name": "BasedGod",
      "images": [
        {
          "src": imgBasedGod,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2412",
      "name": "BASEDWATM8",
      "images": [
        {
          "src": imgBASEDWATM8,
          "fileType": 1,
          "height": 112,
          "width": 110,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2413",
      "name": "BERN",
      "images": [
        {
          "src": imgBERN,
          "fileType": 1,
          "height": 120,
          "width": 180,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2414",
      "name": "BibleThump",
      "images": [
        {
          "src": imgBibleThump,
          "fileType": 1,
          "height": 120,
          "width": 144,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2415",
      "name": "billyWeird",
      "images": [
        {
          "src": imgbillyWeird,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2416",
      "name": "BOGGED",
      "images": [
        {
          "src": imgBOGGED,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2417",
      "name": "BOOMER",
      "images": [
        {
          "src": imgBOOMER,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2418",
      "name": "CampFire",
      "images": [
        {
          "src": imgCampFire,
          "fileType": 1,
          "height": 128,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2419",
      "name": "Cinnabunny",
      "images": [
        {
          "src": imgCinnabunny,
          "fileType": 1,
          "height": 112,
          "width": 560,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 5,
              "durationMs": 500,
              "iterationCount": 7,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2420",
      "name": "cmonBruh",
      "images": [
        {
          "src": imgcmonBruh,
          "fileType": 1,
          "height": 128,
          "width": 122,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2421",
      "name": "ComfyApe",
      "images": [
        {
          "src": imgComfyApe,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2422",
      "name": "ComfyAYA",
      "images": [
        {
          "src": imgComfyAYA,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2423",
      "name": "ComfyCat",
      "images": [
        {
          "src": imgComfyCat,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2424",
      "name": "ComfyDog",
      "images": [
        {
          "src": imgComfyDog,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2425",
      "name": "ComfyFeels",
      "images": [
        {
          "src": imgComfyFeels,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2426",
      "name": "ComfyFerret",
      "images": [
        {
          "src": imgComfyFerret,
          "fileType": 1,
          "height": 128,
          "width": 136,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2427",
      "name": "ComfyPOTATO",
      "images": [
        {
          "src": imgComfyPOTATO,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2428",
      "name": "ComfyWeird",
      "images": [
        {
          "src": imgComfyWeird,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2429",
      "name": "COPIUM",
      "images": [
        {
          "src": imgCOPIUM,
          "fileType": 1,
          "height": 123,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2430",
      "name": "DaFeels",
      "images": [
        {
          "src": imgDaFeels,
          "fileType": 1,
          "height": 128,
          "width": 113,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2431",
      "name": "DAFUK",
      "images": [
        {
          "src": imgDAFUK,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2432",
      "name": "DANKMEMES",
      "images": [
        {
          "src": imgDANKMEMES,
          "fileType": 1,
          "height": 120,
          "width": 288,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2433",
      "name": "DappaKappa",
      "images": [
        {
          "src": imgDappaKappa,
          "fileType": 1,
          "height": 120,
          "width": 104,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2434",
      "name": "DatGeoff",
      "images": [
        {
          "src": imgDatGeoff,
          "fileType": 1,
          "height": 128,
          "width": 114,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2435",
      "name": "DELUSIONAL",
      "images": [
        {
          "src": imgDELUSIONAL,
          "fileType": 1,
          "height": 128,
          "width": 131,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2436",
      "name": "DJPepo",
      "images": [
        {
          "src": imgDJPepo,
          "fileType": 1,
          "height": 91,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2437",
      "name": "DOGGO",
      "images": [
        {
          "src": imgDOGGO,
          "fileType": 1,
          "height": 128,
          "width": 119,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2438",
      "name": "DOIT",
      "images": [
        {
          "src": imgDOIT,
          "fileType": 1,
          "height": 128,
          "width": 183,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2439",
      "name": "DOUBT",
      "images": [
        {
          "src": imgDOUBT,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2440",
      "name": "DuckerZ",
      "images": [
        {
          "src": imgDuckerZ,
          "fileType": 1,
          "height": 104,
          "width": 224,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2441",
      "name": "ECH",
      "images": [
        {
          "src": imgECH,
          "fileType": 1,
          "height": 128,
          "width": 121,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2442",
      "name": "EZ",
      "images": [
        {
          "src": imgEZ,
          "fileType": 1,
          "height": 100,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2443",
      "name": "Facepalm",
      "images": [
        {
          "src": imgFacepalm,
          "fileType": 1,
          "height": 127,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2444",
      "name": "FeelsAmazingMan",
      "images": [
        {
          "src": imgFeelsAmazingMan,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2445",
      "name": "FeelsBadMan",
      "images": [
        {
          "src": imgFeelsBadMan,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2446",
      "name": "FeelsDumbMan",
      "images": [
        {
          "src": imgFeelsDumbMan,
          "fileType": 1,
          "height": 90,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2447",
      "name": "FeelsGoodMan",
      "images": [
        {
          "src": imgFeelsGoodMan,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2448",
      "name": "FeelsOkayMan",
      "images": [
        {
          "src": imgFeelsOkayMan,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2449",
      "name": "FeelsStrongMan",
      "images": [
        {
          "src": imgFeelsStrongMan,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2450",
      "name": "FeelsWeirdMan",
      "images": [
        {
          "src": imgFeelsWeirdMan,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2451",
      "name": "FerretLOL",
      "images": [
        {
          "src": imgFerretLOL,
          "fileType": 1,
          "height": 120,
          "width": 132,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2452",
      "name": "FIDGETLOL",
      "images": [
        {
          "src": imgFIDGETLOL,
          "fileType": 1,
          "height": 100,
          "width": 100,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2453",
      "name": "gachiGASM",
      "images": [
        {
          "src": imggachiGASM,
          "fileType": 1,
          "height": 112,
          "width": 97,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2454",
      "name": "GameOfThrows",
      "images": [
        {
          "src": imgGameOfThrows,
          "fileType": 1,
          "height": 120,
          "width": 316,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2455",
      "name": "GandalfFace",
      "images": [
        {
          "src": imgGandalfFace,
          "fileType": 1,
          "height": 128,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2456",
      "name": "GIMI",
      "images": [
        {
          "src": imgGIMI,
          "fileType": 1,
          "height": 128,
          "width": 144,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2457",
      "name": "GODMAN",
      "images": [
        {
          "src": imgGODMAN,
          "fileType": 1,
          "height": 120,
          "width": 144,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2458",
      "name": "GODWOMAN",
      "images": [
        {
          "src": imgGODWOMAN,
          "fileType": 1,
          "height": 128,
          "width": 213,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2459",
      "name": "GREED",
      "images": [
        {
          "src": imgGREED,
          "fileType": 1,
          "height": 128,
          "width": 121,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2460",
      "name": "haHAA",
      "images": [
        {
          "src": imghaHAA,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2461",
      "name": "HEADSHOT",
      "images": [
        {
          "src": imgHEADSHOT,
          "fileType": 1,
          "height": 120,
          "width": 400,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2462",
      "name": "Hhhehhehe",
      "images": [
        {
          "src": imgHhhehhehe,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2463",
      "name": "Hmmm",
      "images": [
        {
          "src": imgHmmm,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2464",
      "name": "INFESTOR",
      "images": [
        {
          "src": imgINFESTOR,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2465",
      "name": "ITSRAWWW",
      "images": [
        {
          "src": imgITSRAWWW,
          "fileType": 1,
          "height": 128,
          "width": 116,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2466",
      "name": "JimFace",
      "images": [
        {
          "src": imgJimFace,
          "fileType": 1,
          "height": 128,
          "width": 124,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2467",
      "name": "Kappa",
      "images": [
        {
          "src": imgKappa,
          "fileType": 1,
          "height": 128,
          "width": 94,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2468",
      "name": "Klappa",
      "images": [
        {
          "src": imgKlappa,
          "fileType": 1,
          "height": 120,
          "width": 132,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2469",
      "name": "Kreygasm",
      "images": [
        {
          "src": imgKreygasm,
          "fileType": 1,
          "height": 128,
          "width": 99,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2470",
      "name": "LeRuse",
      "images": [
        {
          "src": imgLeRuse,
          "fileType": 1,
          "height": 128,
          "width": 146,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2471",
      "name": "LUL",
      "images": [
        {
          "src": imgLUL,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2472",
      "name": "MASTERB8",
      "images": [
        {
          "src": imgMASTERB8,
          "fileType": 1,
          "height": 128,
          "width": 175,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2473",
      "name": "MiyanoBird",
      "images": [
        {
          "src": imgMiyanoBird,
          "fileType": 1,
          "height": 107,
          "width": 106,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2474",
      "name": "MiyanoComfy",
      "images": [
        {
          "src": imgMiyanoComfy,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2475",
      "name": "MLADY",
      "images": [
        {
          "src": imgMLADY,
          "fileType": 1,
          "height": 109,
          "width": 104,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2476",
      "name": "monkaHmm",
      "images": [
        {
          "src": imgmonkaHmm,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2477",
      "name": "monkaMEGA",
      "images": [
        {
          "src": imgmonkaMEGA,
          "fileType": 1,
          "height": 111,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2478",
      "name": "monkaS",
      "images": [
        {
          "src": imgmonkaS,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2479",
      "name": "monkaVirus",
      "images": [
        {
          "src": imgmonkaVirus,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2480",
      "name": "MOOBERS",
      "images": [
        {
          "src": imgMOOBERS,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2481",
      "name": "MotherFuckinGame",
      "images": [
        {
          "src": imgMotherFuckinGame,
          "fileType": 1,
          "height": 128,
          "width": 127,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2484",
      "name": "NeneLaugh",
      "images": [
        {
          "src": imgNeneLaugh,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2485",
      "name": "NiceMeMe",
      "images": [
        {
          "src": imgNiceMeMe,
          "fileType": 1,
          "height": 128,
          "width": 86,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2486",
      "name": "NOBULLY",
      "images": [
        {
          "src": imgNOBULLY,
          "fileType": 1,
          "height": 128,
          "width": 125,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2487",
      "name": "NOM",
      "images": [
        {
          "src": imgNOM,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2488",
      "name": "NoTears",
      "images": [
        {
          "src": imgNoTears,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2489",
      "name": "NotLikeThis",
      "images": [
        {
          "src": imgNotLikeThis,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2490",
      "name": "NOTMYTEMPO",
      "images": [
        {
          "src": imgNOTMYTEMPO,
          "fileType": 1,
          "height": 120,
          "width": 92,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2491",
      "name": "OBJECTION",
      "images": [
        {
          "src": imgOBJECTION,
          "fileType": 1,
          "height": 128,
          "width": 141,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2492",
      "name": "OHDEAR",
      "images": [
        {
          "src": imgOHDEAR,
          "fileType": 1,
          "height": 128,
          "width": 106,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2493",
      "name": "OMEGALUL",
      "images": [
        {
          "src": imgOMEGALUL,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2494",
      "name": "OnlyPretending",
      "images": [
        {
          "src": imgOnlyPretending,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2495",
      "name": "ORDAH",
      "images": [
        {
          "src": imgORDAH,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2496",
      "name": "OsKrappa",
      "images": [
        {
          "src": imgOsKrappa,
          "fileType": 1,
          "height": 128,
          "width": 144,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2497",
      "name": "OSTRIGGERED",
      "images": [
        {
          "src": imgOSTRIGGERED,
          "fileType": 1,
          "height": 128,
          "width": 206,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2498",
      "name": "OverRustle",
      "images": [
        {
          "src": imgOverRustle,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2499",
      "name": "PAIN",
      "images": [
        {
          "src": imgPAIN,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2500",
      "name": "PauseChamp",
      "images": [
        {
          "src": imgPauseChamp,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2501",
      "name": "PeepoHappy",
      "images": [
        {
          "src": imgPeepoHappy,
          "fileType": 1,
          "height": 79,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2502",
      "name": "PeepoRiot",
      "images": [
        {
          "src": imgPeepoRiot,
          "fileType": 1,
          "height": 86,
          "width": 124,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2503",
      "name": "PeepoS",
      "images": [
        {
          "src": imgPeepoS,
          "fileType": 1,
          "height": 128,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2504",
      "name": "peepoWave",
      "images": [
        {
          "src": imgpeepoWave,
          "fileType": 1,
          "height": 119,
          "width": 119,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2505",
      "name": "PeepoWeird",
      "images": [
        {
          "src": imgPeepoWeird,
          "fileType": 1,
          "height": 79,
          "width": 111,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2506",
      "name": "PEPE",
      "images": [
        {
          "src": imgPEPE,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2507",
      "name": "PepeComfy",
      "images": [
        {
          "src": imgPepeComfy,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2508",
      "name": "Pepega",
      "images": [
        {
          "src": imgPepega,
          "fileType": 1,
          "height": 98,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2509",
      "name": "PepeHands",
      "images": [
        {
          "src": imgPepeHands,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2510",
      "name": "PepeLaugh",
      "images": [
        {
          "src": imgPepeLaugh,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2511",
      "name": "PepeMods",
      "images": [
        {
          "src": imgPepeMods,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2512",
      "name": "PepoBan",
      "images": [
        {
          "src": imgPepoBan,
          "fileType": 1,
          "height": 128,
          "width": 160,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2513",
      "name": "PepoComfy",
      "images": [
        {
          "src": imgPepoComfy,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2514",
      "name": "PepoG",
      "images": [
        {
          "src": imgPepoG,
          "fileType": 1,
          "height": 103,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2515",
      "name": "PepoGood",
      "images": [
        {
          "src": imgPepoGood,
          "fileType": 1,
          "height": 128,
          "width": 206,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2516",
      "name": "PepoHmm",
      "images": [
        {
          "src": imgPepoHmm,
          "fileType": 1,
          "height": 89,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2517",
      "name": "PepOk",
      "images": [
        {
          "src": imgPepOk,
          "fileType": 1,
          "height": 128,
          "width": 146,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2518",
      "name": "PepoPirate",
      "images": [
        {
          "src": imgPepoPirate,
          "fileType": 1,
          "height": 128,
          "width": 142,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2519",
      "name": "PepoSleep",
      "images": [
        {
          "src": imgPepoSleep,
          "fileType": 1,
          "height": 128,
          "width": 141,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2520",
      "name": "PepoThink",
      "images": [
        {
          "src": imgPepoThink,
          "fileType": 1,
          "height": 122,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2521",
      "name": "PepoWant",
      "images": [
        {
          "src": imgPepoWant,
          "fileType": 1,
          "height": 88,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2522",
      "name": "PIKOHH",
      "images": [
        {
          "src": imgPIKOHH,
          "fileType": 1,
          "height": 100,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2523",
      "name": "POGGERS",
      "images": [
        {
          "src": imgPOGGERS,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2524",
      "name": "POGGIES",
      "images": [
        {
          "src": imgPOGGIES,
          "fileType": 1,
          "height": 106,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2525",
      "name": "POGOI",
      "images": [
        {
          "src": imgPOGOI,
          "fileType": 1,
          "height": 112,
          "width": 110,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2526",
      "name": "POKE",
      "images": [
        {
          "src": imgPOKE,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2527",
      "name": "POTATO",
      "images": [
        {
          "src": imgPOTATO,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2528",
      "name": "POUT",
      "images": [
        {
          "src": imgPOUT,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2529",
      "name": "QUEEN",
      "images": [
        {
          "src": imgQUEEN,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2530",
      "name": "RedCard",
      "images": [
        {
          "src": imgRedCard,
          "fileType": 1,
          "height": 128,
          "width": 88,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2532",
      "name": "Riperino",
      "images": [
        {
          "src": imgRiperino,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2533",
      "name": "sataniaLUL",
      "images": [
        {
          "src": imgsataniaLUL,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2534",
      "name": "SEMPAI",
      "images": [
        {
          "src": imgSEMPAI,
          "fileType": 1,
          "height": 128,
          "width": 144,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2535",
      "name": "SHOCK",
      "images": [
        {
          "src": imgSHOCK,
          "fileType": 1,
          "height": 128,
          "width": 133,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2536",
      "name": "SHRUG",
      "images": [
        {
          "src": imgSHRUG,
          "fileType": 1,
          "height": 115,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2537",
      "name": "shyLurk",
      "images": [
        {
          "src": imgshyLurk,
          "fileType": 1,
          "height": 63,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2538",
      "name": "SICKOOH",
      "images": [
        {
          "src": imgSICKOOH,
          "fileType": 1,
          "height": 128,
          "width": 184,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2539",
      "name": "Sippy",
      "images": [
        {
          "src": imgSippy,
          "fileType": 1,
          "height": 128,
          "width": 116,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2540",
      "name": "SLEEPY",
      "images": [
        {
          "src": imgSLEEPY,
          "fileType": 1,
          "height": 128,
          "width": 231,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2541",
      "name": "SMOrc",
      "images": [
        {
          "src": imgSMOrc,
          "fileType": 1,
          "height": 128,
          "width": 123,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2542",
      "name": "SMUG",
      "images": [
        {
          "src": imgSMUG,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2543",
      "name": "SoDoge",
      "images": [
        {
          "src": imgSoDoge,
          "fileType": 1,
          "height": 128,
          "width": 189,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2544",
      "name": "SOTRIGGERED",
      "images": [
        {
          "src": imgSOTRIGGERED,
          "fileType": 1,
          "height": 119,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2545",
      "name": "SpookerZ",
      "images": [
        {
          "src": imgSpookerZ,
          "fileType": 1,
          "height": 128,
          "width": 136,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2546",
      "name": "SPY",
      "images": [
        {
          "src": imgSPY,
          "fileType": 1,
          "height": 128,
          "width": 138,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2547",
      "name": "SUGOI",
      "images": [
        {
          "src": imgSUGOI,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2548",
      "name": "SUGOwO",
      "images": [
        {
          "src": imgSUGOwO,
          "fileType": 1,
          "height": 120,
          "width": 120,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2549",
      "name": "SURPRISE",
      "images": [
        {
          "src": imgSURPRISE,
          "fileType": 1,
          "height": 120,
          "width": 132,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2550",
      "name": "SWEATY",
      "images": [
        {
          "src": imgSWEATY,
          "fileType": 1,
          "height": 128,
          "width": 141,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2551",
      "name": "TIMID",
      "images": [
        {
          "src": imgTIMID,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2552",
      "name": "umaruCry",
      "images": [
        {
          "src": imgumaruCry,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2553",
      "name": "UWOTM8",
      "images": [
        {
          "src": imgUWOTM8,
          "fileType": 1,
          "height": 128,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2554",
      "name": "WEEWOO",
      "images": [
        {
          "src": imgWEEWOO,
          "fileType": 1,
          "height": 120,
          "width": 132,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2555",
      "name": "weSmart",
      "images": [
        {
          "src": imgweSmart,
          "fileType": 1,
          "height": 112,
          "width": 109,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2556",
      "name": "WhoahDude",
      "images": [
        {
          "src": imgWhoahDude,
          "fileType": 1,
          "height": 128,
          "width": 97,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2557",
      "name": "WICKED",
      "images": [
        {
          "src": imgWICKED,
          "fileType": 1,
          "height": 86,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2558",
      "name": "Woof",
      "images": [
        {
          "src": imgWoof,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2559",
      "name": "Wowee",
      "images": [
        {
          "src": imgWowee,
          "fileType": 1,
          "height": 108,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2560",
      "name": "YEE",
      "images": [
        {
          "src": imgYEE,
          "fileType": 1,
          "height": 112,
          "width": 112,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2561",
      "name": "YellowCard",
      "images": [
        {
          "src": imgYellowCard,
          "fileType": 1,
          "height": 128,
          "width": 88,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2562",
      "name": "ZOOMER",
      "images": [
        {
          "src": imgZOOMER,
          "fileType": 1,
          "height": 128,
          "width": 128,
          "scale": 2,
        },
      ],
      "effects": [],
    },
    {
      "id": "2563",
      "name": "RIDIN",
      "images": [
        {
          "src": imgRIDIN,
          "fileType": 1,
          "height": 128,
          "width": 2560,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 8,
              "durationMs": 2000,
              "iterationCount": 7,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2564",
      "name": "TANTIES",
      "images": [
        {
          "src": imgTANTIES,
          "fileType": 1,
          "height": 112,
          "width": 1120,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 10,
              "durationMs": 1400,
              "iterationCount": 5,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2565",
      "name": "VroomVroom",
      "images": [
        {
          "src": imgVroomVroom,
          "fileType": 1,
          "height": 112,
          "width": 11312,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 101,
              "durationMs": 4000,
              "iterationCount": 2,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2566",
      "name": "WAG",
      "images": [
        {
          "src": imgWAG,
          "fileType": 1,
          "height": 128,
          "width": 296,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 2,
              "durationMs": 500,
              "iterationCount": 10,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2567",
      "name": "WAYTOODANK",
      "images": [
        {
          "src": imgWAYTOODANK,
          "fileType": 1,
          "height": 112,
          "width": 10080,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 90,
              "durationMs": 1800,
              "iterationCount": 2,
              "loopForever": true,
              "endOnFrame": 45,
            },
          },
        },
      ],
    },
    {
      "id": "2568",
      "name": "BONK",
      "images": [
        {
          "src": imgBONK,
          "fileType": 1,
          "height": 128,
          "width": 440,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 2,
              "durationMs": 500,
              "iterationCount": 10,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2569",
      "name": "catJAM",
      "images": [
        {
          "src": imgcatJAM,
          "fileType": 1,
          "height": 112,
          "width": 17696,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 158,
              "durationMs": 6500,
              "iterationCount": 2,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2570",
      "name": "ComfyMoobers",
      "images": [
        {
          "src": imgComfyMoobers,
          "fileType": 1,
          "height": 128,
          "width": 4352,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 34,
              "durationMs": 2550,
              "iterationCount": 2,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2571",
      "name": "DuckJAM",
      "images": [
        {
          "src": imgDuckJAM,
          "fileType": 1,
          "height": 112,
          "width": 6160,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 55,
              "durationMs": 5500,
              "iterationCount": 1,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2572",
      "name": "NODDERS",
      "images": [
        {
          "src": imgNODDERS,
          "fileType": 1,
          "height": 112,
          "width": 448,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 4,
              "durationMs": 320,
              "iterationCount": 16,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2573",
      "name": "NOPERS",
      "images": [
        {
          "src": imgNOPERS,
          "fileType": 1,
          "height": 112,
          "width": 896,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 8,
              "durationMs": 640,
              "iterationCount": 16,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2574",
      "name": "PeepoRun",
      "images": [
        {
          "src": imgPeepoRun,
          "fileType": 1,
          "height": 112,
          "width": 672,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 6,
              "durationMs": 420,
              "iterationCount": 12,
              "loopForever": true,
            },
          },
        },
      ],
    },
    {
      "id": "2531",
      "name": "REE",
      "images": [
        {
          "src": imgREE,
          "fileType": 1,
          "height": 112,
          "width": 4992,
          "scale": 2,
        },
      ],
      "effects": [
        {
          "effect": {
            "spriteAnimation": {
              "frameCount": 39,
              "durationMs": 3900,
              "iterationCount": 2,
              "loopForever": true,
              "alternateDirection": true,
            },
          },
        },
        {
          "effect": {
            "defaultModifiers": {
              "modifiers": ["rustle"],
            },
          },
        },
      ],
    },
  ],
  "tags": [
    {
      "id": "4000",
      "name": "nsfw",
    },
    {
      "id": "4001",
      "name": "loud",
    },
    {
      "id": "4002",
      "name": "weeb",
    },
    {
      "id": "4003",
      "name": "nsfl",
    },
  ],
  "modifiers": [
    {
      "id": "3000",
      "name": "mirror",
    },
    {
      "id": "3001",
      "name": "smol",
    },
    {
      "id": "3002",
      "name": "flip",
    },
    {
      "id": "3003",
      "name": "rain",
    },
    {
      "id": "3004",
      "name": "snow",
    },
    {
      "id": "3005",
      "name": "rustle",
    },
    {
      "id": "3006",
      "name": "worth",
    },
    {
      "id": "3007",
      "name": "dank",
    },
    {
      "id": "3008",
      "name": "hyper",
    },
    {
      "id": "3009",
      "name": "love",
    },
    {
      "id": "3010",
      "name": "spin",
    },
    {
      "id": "3011",
      "name": "wide",
    },
    // {
    //   "id": "3012",
    //   "name": "virus",
    // },
    {
      "id": "3013",
      "name": "banned",
    },
    {
      "id": "3014",
      "name": "lag",
    },
    {
      "id": "3015",
      "name": "pause",
    },
    {
      "id": "3016",
      "name": "slow",
    },
    {
      "id": "3017",
      "name": "fast",
    },
    {
      "id": "3018",
      "name": "reverse",
    },
    {
      "id": "3019",
      "name": "jam",
    },
    {
      "id": "3020",
      "name": "pride",
    },
  ],
  "removedIds": [],
};

export default async (): Promise<chatv1.AssetBundle> => {
  return new chatv1.AssetBundle({
    ...src,
    emotes: await Promise.all(
      src.emotes.map(async ({ id, images, ...emote }) => {
        return new chatv1.Emote({
          ...emote,
          id: BigInt(id),
          images: await Promise.all(
            images.map(async ({ src, ...image }) => {
              const res = await fetch(src);
              return new chatv1.EmoteImage({
                ...image,
                data: new Uint8Array(await res.arrayBuffer()),
              });
            })
          ),
        });
      })
    ),
    tags: src.tags.map(({ id, ...tag }) => ({
      ...tag,
      id: BigInt(id),
    })),
    modifiers: src.modifiers.map(({ id, ...modifier }) => ({
      ...modifier,
      id: BigInt(id),
    })),
  });
};
