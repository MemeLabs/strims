// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Message } from "../../../apis/strims/chat/v1/chat";

const history = [
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089095563,"data":"Idk about the others","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089115441,"data":"abesus that might be one of the funniest things I\'ve seen in a long time","entities":{"nicks":[{"nick":"abesus","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089135089,"data":"The Jurassic World pokimane gif","entities":{}}',
  'MSG {"nick":"abesus","features":[],"timestamp":1653089175958,"data":"soadsoap its a classic","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089176538,"data":"KartoffelKopf widdlez I wonder how many more of these are coming that he is saying this shit on twitter lmao","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}},{"nick":"widdlez","bounds":{"start":14,"end":21}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089246835,"data":"soadsoap  more of what?","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089253478,"data":"oh the sexual assault claims","entities":{}}',
  'MSG {"nick":"Of_Odin","features":[],"timestamp":1653089260181,"data":"soadsoap bezos is trying to go for twitter dunks too","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089275632,"data":"KartoffelKopf yeah","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089277855,"data":"bezos is gonna run","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089285828,"data":"KartoffelKopf there is no chance that was a one-off","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089291920,"data":"widdlez  PeepoRun ?","entities":{"emotes":[{"name":"PeepoRun","bounds":{"start":9,"end":17}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089292024,"data":"For president?","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089294736,"data":"What\'d he say","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089298521,"data":"either that or hes just also mad about the union memes","entities":{}}',
  'MSG {"nick":"Of_Odin","features":[],"timestamp":1653089299354,"data":"not sure if gunshots or car exhast PepoThink","entities":{"emotes":[{"name":"PepoThink","bounds":{"start":35,"end":44}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089308818,"data":"Run boy run PeepoRun they\'re trying to catch you PeepoRun","entities":{"emotes":[{"name":"PeepoRun","bounds":{"start":12,"end":20}},{"name":"PeepoRun","bounds":{"start":49,"end":57}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089309635,"data":"soadsoap hes been tweeting a bunch more politics stuff lately","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089316755,"data":"soadsoap critical of biden stuff","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089349173,"data":"widdlez can he realistically run as a Republican with all the initiatives he\'s pushing about climate change etc LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":112,"end":115}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089351570,"data":"Oh god","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089356211,"data":"Hell world","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089363295,"data":"Worst timeline","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089367086,"data":"Only getting worse","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089388177,"data":"Jeff Bezos (R) vs Mark Cuban (D) 2024","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089391511,"data":"soadsoap i doubt he\'d run republican","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089400390,"data":"soadsoap hes probably just TANTIES ing about amazon union efforts anyway PepeLaugh","entities":{"emotes":[{"name":"TANTIES","bounds":{"start":27,"end":34}},{"name":"PepeLaugh","bounds":{"start":73,"end":82}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089400840,"data":"widdlez he ain\'t running as a dem that\'s for sure","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089405889,"data":"Yeah probably","entities":{}}',
  'MSG {"nick":"biscophan","features":[],"timestamp":1653089407696,"data":"i kinda want cuban to run for a texas goverment seat","entities":{}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089426202,"data":"widdlez doesn\'t sounds too bad right now but he\'s only just started going full retard so we\'ll see","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"biscophan","features":[],"timestamp":1653089434821,"data":"lt gov., gov, maybe comptroller","entities":{}}',
  'MSG {"nick":"jbpratt","features":["moderator"],"timestamp":1653089438823,"data":"KartoffelKopf billyWeird don\'t use the r word here","entities":{"emotes":[{"name":"billyWeird","bounds":{"start":14,"end":24}}],"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089452444,"data":"KartoffelKopf the only r word we want from you","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089453670,"data":"is roden","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089454478,"data":"t","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089455625,"data":"FerretLOL","entities":{"emotes":[{"name":"FerretLOL","bounds":{"start":0,"end":9}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089474072,"data":"ComfyFerret","entities":{"emotes":[{"name":"ComfyFerret","bounds":{"start":0,"end":11}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089484475,"data":"biscophan he would be much better for Texas than the country writ large I think but I truly hope this era of corporately rich people with zero political experience running for high office ends","entities":{"nicks":[{"nick":"biscophan","bounds":{"start":0,"end":9}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653089506598,"data":"tahley bawrroccoli BOOMIES","entities":{"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}},{"nick":"bawrroccoli","bounds":{"start":7,"end":18}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089507428,"data":"soadsoap  it\'s only just beginning PEPE","entities":{"emotes":[{"name":"PEPE","bounds":{"start":35,"end":39}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089529078,"data":"was there anyone like that before trump?","entities":{}}',
  'MSG {"nick":"jbpratt","features":["moderator"],"timestamp":1653089539470,"data":"KartoffelKopf arnold","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089546214,"data":"biscophan I know it\'s happened in the past and isn\'t unique to now but when I see the Rock getting floated for president I want to cry","entities":{"nicks":[{"nick":"biscophan","bounds":{"start":0,"end":9}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089548738,"data":"cuz we got bloomberg too now and bezos or zuckerburg maybe","entities":{}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089563375,"data":"jbpratt arnold?","entities":{"nicks":[{"nick":"jbpratt","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089565462,"data":"KartoffelKopf Reagan was an actor then became governor of CA","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089576107,"data":"KartoffelKopf Schwarzenegger","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089580783,"data":"soadsoap  yeah not a billionaire though","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089601542,"data":"Still a mega rich guy with no political experience","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089624408,"data":"Bloomberg was mayor of NYC for a long time too so it\'s not like he had zero but still bad","entities":{}}',
  'MSG {"nick":"biscophan","features":[],"timestamp":1653089651314,"data":"soadsoap god i could see the rock running for office only to push his energy drink or some shit","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089666423,"data":"Arnie was a pretty good governor I think like he was popular and had some impactful policies if I remember correctly","entities":{}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089673927,"data":"maybe one day I can make a billion dollars and finally get elected class president FeelsStrongMan","entities":{"emotes":[{"name":"FeelsStrongMan","bounds":{"start":83,"end":97}}]}}',
  'MSG {"nick":"jbpratt","features":["moderator"],"timestamp":1653089699671,"data":"KartoffelKopf the net worth difference between Trump and Schwarzenegger is less than the difference between Trump and Bezos","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089702929,"data":"KartoffelKopf just run as a joke candidate in an era of massive political dissatisfaction","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089711395,"data":"biscophan we stay hungry NODDERS we devour NODDERS put in the work NODDERS takes whats ours NODDERS the white house is next NODDERS","entities":{"emotes":[{"name":"NODDERS","bounds":{"start":25,"end":32}},{"name":"NODDERS","bounds":{"start":43,"end":50}},{"name":"NODDERS","bounds":{"start":67,"end":74}},{"name":"NODDERS","bounds":{"start":92,"end":99}},{"name":"NODDERS","bounds":{"start":124,"end":131}}],"nicks":[{"nick":"biscophan","bounds":{"start":0,"end":9}}]}}',
  'MSG {"nick":"biscophan","features":[],"timestamp":1653089716660,"data":"soadsoap i think he could have a chance and convince people in texas that he\'s suited for the comptroller position. gov or lt gov position not likely. he\'s like a new age ross perot person","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"Of_Odin","features":[],"timestamp":1653089721402,"data":"soadsoap I\'ve read the opposite","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089730388,"data":"KartoffelKopf some english town elected a mascot as their mayor and he turned out to be really good","entities":{"nicks":[{"nick":"KartoffelKopf","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"biscophan","features":[],"timestamp":1653089730981,"data":"soadsoap NODDERS","entities":{"emotes":[{"name":"NODDERS","bounds":{"start":9,"end":16}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089733473,"data":"so it\'s not unheard of","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089738584,"data":"many such examples","entities":{}}',
  'MSG {"nick":"Of_Odin","features":[],"timestamp":1653089741216,"data":"soadsoap he was terrible at start but then late into his term he was better","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089751474,"data":"the first female elected official in the US was put on the ballot as a joke","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089753704,"data":"and she did well","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089773350,"data":"I like the \\"dog for mayor\\" ballot bit","entities":{}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653089775477,"data":"tahley seems like the joke was on the haters","entities":{"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089792756,"data":"I\'d vote for a dog Wowee","entities":{"emotes":[{"name":"Wowee","bounds":{"start":19,"end":24}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089794503,"data":"maybe we\'ll see a president FerretLOL  in our life time","entities":{"emotes":[{"name":"FerretLOL","bounds":{"start":28,"end":37}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089795376,"data":"Gehirnchirurg ye it was a \\" elyouel a woman could never win let\'s put her on it\\"","entities":{"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089801676,"data":"soadsoap a lot of animals are mayors","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089802338,"data":"i\'d vote for a FerretLOL","entities":{"emotes":[{"name":"FerretLOL","bounds":{"start":15,"end":24}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089804715,"data":"cats, dogs, horses","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089808304,"data":"monkeys","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089815870,"data":"tahley yeah it\'s cute","entities":{"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089820018,"data":"wasnt there a big meme with arnold before he left office","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089825649,"data":"soadsoap until you realize that position does actually have a purpose","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089833570,"data":"like he commuted some guys sentence as a favor and the guy was in jail for murder LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":82,"end":85}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089843043,"data":"as Nept will attest to with caligula putting a horse as his advisor or something? i forgot the position","entities":{"nicks":[{"nick":"Nept","bounds":{"start":3,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089845797,"data":"tahley well yeah PepeLaugh but usually it\'s a symbolic thing with no repercussions right","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":17,"end":26}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"KartoffelKopf","features":[],"timestamp":1653089847102,"data":"widdlez  he freed his boy","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089849542,"data":"basically the only position that could overrule the caesar","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653089853690,"data":"https://en.wikipedia.org/wiki/Death_of_Luis_Santos PepoG `As a personal favor to “a friend”, just hours before he left office, and as one of his last official acts, Schwarzenegger commuted Núñez’s sentence by more than half, to seven years`","entities":{"links":[{"url":"https://en.wikipedia.org/wiki/Death_of_Luis_Santos","bounds":{"start":0,"end":50}}],"emotes":[{"name":"PepoG","bounds":{"start":51,"end":56}}],"codes":[{"bounds":{"start":57,"end":240}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653089870220,"data":"tahley https://en.wikipedia.org/wiki/Incitatus","entities":{"links":[{"url":"https://en.wikipedia.org/wiki/Incitatus","bounds":{"start":7,"end":46}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089884202,"data":"Nept ye the consul","entities":{"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653089906463,"data":"Nept which ye is just a meme about him being mad and not real i guess PepoThink","entities":{"emotes":[{"name":"PepoThink","bounds":{"start":70,"end":79}}],"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089906795,"data":"Nept tahley and what of good Horselonius? �","entities":{"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}},{"nick":"tahley","bounds":{"start":5,"end":11}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653089914750,"data":"soadsoap OMEGALUL OMEGALUL OMEGALUL","entities":{"emotes":[{"name":"OMEGALUL","bounds":{"start":9,"end":17}},{"name":"OMEGALUL","bounds":{"start":18,"end":26}},{"name":"OMEGALUL","bounds":{"start":27,"end":35}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089932234,"data":"Nept thank you I needed that PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":29,"end":38}}],"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653089946937,"data":"soadsoap it was a good one FeelsOkayMan","entities":{"emotes":[{"name":"FeelsOkayMan","bounds":{"start":27,"end":39}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653089960686,"data":"Nept ty sggL I\'m still laughing PepeLaugh","entities":{"emotes":[{"name":"sggL","bounds":{"start":8,"end":12}},{"name":"PepeLaugh","bounds":{"start":32,"end":41}}],"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090067516,"data":"https://youtu.be/jgTh7qvFRNk?t=708 BOOMIES EEEEEEE","entities":{"links":[{"url":"https://youtu.be/jgTh7qvFRNk?t=708","bounds":{"start":0,"end":34}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090076285,"data":"BOOMIES E BOOMIES E BOOMIES E BOOMIES E BOOMIES E","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090165958,"data":"how did they manage to make a sequel 5y later that looks far worse LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":67,"end":70}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090205558,"data":"widdlez reading through it it seems the guy he commuted the sentence for didn\'t actually kill anyone but that is still really bad it seems LUL the optics alone at least","entities":{"emotes":[{"name":"LUL","bounds":{"start":139,"end":142}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090227757,"data":"widdlez he stabbed someone else during a fight but the guy who died was stabbed by a different person","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090252441,"data":"Here\'s a hot take","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090266732,"data":"Bruce Willis is still an asshole for roles he is currently doing even though he has dementia","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090276247,"data":"Ph4t3 Jason Statham movies are quick and dirty. They print money. It\'s not hard to convince someone to make a movie with him","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653090277743,"data":"tahley PepoHmm","entities":{"emotes":[{"name":"PepoHmm","bounds":{"start":7,"end":14}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090297948,"data":"soadsoap his dad was the speaker of the california state assembly at the time LUL curious PEPE","entities":{"emotes":[{"name":"LUL","bounds":{"start":78,"end":81}},{"name":"PEPE","bounds":{"start":90,"end":94}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090309640,"data":"tahley monkaHmm why is he an asshole for doing shitty movies tho","entities":{"emotes":[{"name":"monkaHmm","bounds":{"start":7,"end":15}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090315165,"data":"soadsoap GREED","entities":{"emotes":[{"name":"GREED","bounds":{"start":9,"end":14}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090320948,"data":"soadsoap didn\'t know he sells that well","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090323439,"data":"tahley I think it\'s more appropriate to say he\'s an asshole his entire career and his dementia doesn\'t change that but this latter part of his career should probably not be bullied","entities":{"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090339384,"data":"widdlez because he takes 99% of the budget and basically fuck over the entire production","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090356349,"data":"That\'s on the people who make the movies though","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090359216,"data":"Not on him","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090380403,"data":"soadsoap but thinking about it now I see, there\'s not many action stars left that just sell a movie with their presence to fulfill some BOOMER fantasies","entities":{"emotes":[{"name":"BOOMER","bounds":{"start":136,"end":142}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090405413,"data":"widdlez yeah that\'s what I mean by the optics alone LUL it\'s Arnie\'s close friend and \\"staunch ally\\" LUL he knew it was fucked up which is why he did it an hour before he left office","entities":{"emotes":[{"name":"LUL","bounds":{"start":52,"end":55}},{"name":"LUL","bounds":{"start":101,"end":104}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090440443,"data":"soadsoap also he apparently fucked education funding but im too lazy to actually research that NoTears","entities":{"emotes":[{"name":"NoTears","bounds":{"start":95,"end":102}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090450863,"data":"Ph4t3 the Rock and Vin Diesel still do that right, and so does Statham LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":71,"end":74}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090470351,"data":"widdlez I\'m not married to the position he was a good governor, just what I remember reading","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090483936,"data":"soadsoap hence the success of the fast and furious franchise PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":61,"end":70}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090496832,"data":"widdlez he did some good things with renewables and climate change stuff if I recall","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653090519979,"data":"I think you do some fucked up things and still be a good governor overall. no idea if arnie fits that bill though","entities":{}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653090547647,"data":"he is austrian, so I am naturally inclined to be wary of him PEPE","entities":{"emotes":[{"name":"PEPE","bounds":{"start":61,"end":65}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090548534,"data":"Gehirnchirurg no TANTIES this is the internet in 2022","entities":{"emotes":[{"name":"TANTIES","bounds":{"start":17,"end":24}}],"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090553609,"data":"Gehirnchirurg one bad thing AND I HATE YOU TANTIES","entities":{"emotes":[{"name":"TANTIES","bounds":{"start":43,"end":50}}],"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090561344,"data":"There is no political office holder who ever lived that did not do some bad while also doing some good","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090566004,"data":"It\'s the nature of the beast","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090634556,"data":"Ph4t3 also I feel like the \\"action movie\\" from years past is now just replaced with Marvel movies","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653090654961,"data":"the education fund thing would be forgiveable yeah. the reducing sentence thing feels a bit more weird. but I guess its the US system, its legal so SHRUG","entities":{"emotes":[{"name":"SHRUG","bounds":{"start":148,"end":153}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090666721,"data":"Ph4t3 so guys like Hemsworth and Chris Pratt etc are still pretty prototypical action movie stars","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090693082,"data":"Mark Wahlberg is definitely a shitty generic action movie star still","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090710729,"data":"The rock is too","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090721526,"data":"but i guess he makes more kids movies PepoThink","entities":{"emotes":[{"name":"PepoThink","bounds":{"start":38,"end":47}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090755831,"data":"tahley YEE I said him before","entities":{"emotes":[{"name":"YEE","bounds":{"start":7,"end":10}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090764488,"data":"The rock, Jason Statham, Vin Diesel","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090769731,"data":"soadsoap oh sorry","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090775015,"data":"Not a problem","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090785514,"data":"statham seems to have disappeared no PepoThink","entities":{"emotes":[{"name":"PepoThink","bounds":{"start":37,"end":46}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090801585,"data":"Idk","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090807552,"data":"https://www.imdb.com/name/nm0005458/ oh he does fast movies","entities":{"links":[{"url":"https://www.imdb.com/name/nm0005458/","bounds":{"start":0,"end":36}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090812439,"data":"He is now in the fast and furious spinoff series right","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090814874,"data":"i forgot about that","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090815998,"data":"With the rock","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090825951,"data":"And he just did another action movie recent","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090830932,"data":"Can\'t remember the name","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090834153,"data":"tahley soadsoap also the last Guy Ritchie movie which was absolute trash PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":73,"end":82}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}},{"nick":"soadsoap","bounds":{"start":7,"end":15}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090834847,"data":"how could you forget about bob and hoss","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090836397,"data":"Wrath of Man or some shit","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090843655,"data":"soadsoap yeah that one","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090844787,"data":"Was that it? Ph4t3","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":13,"end":18}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653090844906,"data":"i mean hobby and shobby","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090849238,"data":"YEE okay","entities":{"emotes":[{"name":"YEE","bounds":{"start":0,"end":3}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090852625,"data":"i\'ll be honest","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090863298,"data":"i haevn\'t seen a single f\\u0026f movie outside tokyo drift","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090869837,"data":"I feel like his star is a bit reduced from the 2000s and 2010s but he still is a big draw","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090881767,"data":"tahley missing out on the best superhero franchise of our current time PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":71,"end":80}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"blankspaceblank","features":[],"timestamp":1653090883694,"data":"Action movies are not what they used to be BOOMER","entities":{"emotes":[{"name":"BOOMER","bounds":{"start":43,"end":49}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090886612,"data":"He did all the expendables movies too actually","entities":{}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090891349,"data":"Ph4t3 no i saw morbius","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090892683,"data":"Nah this dude is still a big boy nvm","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090908253,"data":"He just bounces from one money printing franchise to another","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090923942,"data":"tahley I don\'t think I\'ll ever watch that PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":42,"end":51}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090931989,"data":"blankspaceblank they\'re just comic book movies now","entities":{"nicks":[{"nick":"blankspaceblank","bounds":{"start":0,"end":15}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653090935356,"data":"Ph4t3 DELUSIONAL everyone saw it. It sweeped the nation","entities":{"emotes":[{"name":"DELUSIONAL","bounds":{"start":6,"end":16}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090949965,"data":"tahley I hate the fast and furious franchise more than I can put into words","entities":{"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090953073,"data":"tahley not my nation NoTears","entities":{"emotes":[{"name":"NoTears","bounds":{"start":21,"end":28}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653090963645,"data":"tahley last thing that swept this nation went a bit overboard PEPE","entities":{"emotes":[{"name":"PEPE","bounds":{"start":62,"end":66}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"blankspaceblank","features":[],"timestamp":1653090975480,"data":"oh yea","entities":{}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653090987317,"data":"Ph4t3 reading the synopsis of wrath of man, it does read like a statham action b-movie in a nuthsell LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":101,"end":104}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653090991010,"data":"Ph4t3 the soviets did go a bit ham sweeping over Germany true PEPE","entities":{"emotes":[{"name":"PEPE","bounds":{"start":62,"end":66}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091010186,"data":"Gehirnchirurg Ph4t3 yeah it\'s just easy money machine go brrrrrr","entities":{"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}},{"nick":"Ph4t3","bounds":{"start":14,"end":19}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091033160,"data":"You spend minimal effort on a story and writing etc and just film Statham doing dumb shit and double your investment","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091039383,"data":"Gehirnchirurg it\'s an actual trash movie, I watched it with a friend and we just meme\'d it to death PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":100,"end":109}}],"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091084983,"data":"They spent $40m to make at least $104m","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091088634,"data":"Brrrrr","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091095839,"data":"she mispronounced phnom penh as \'Penang Penh\' PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":46,"end":55}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091097704,"data":"good job","entities":{}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653091122194,"data":"Ph4t3 she would pronounce your name like Ftwpala did NODDERS","entities":{"emotes":[{"name":"NODDERS","bounds":{"start":53,"end":60}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091168456,"data":"Gehirnchirurg cause her speech would obstructed PEPE","entities":{"emotes":[{"name":"PEPE","bounds":{"start":48,"end":52}}],"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091171149,"data":"You mean Ph4t3 s name isn\'t pronounced \\"ffff-forty-three\\"? PIKOHH","entities":{"emotes":[{"name":"PIKOHH","bounds":{"start":59,"end":65}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":9,"end":14}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091185977,"data":"SPY","entities":{"emotes":[{"name":"SPY","bounds":{"start":0,"end":3}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091190453,"data":"now this is why I tuned in","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091202489,"data":"too short TANTIES","entities":{"emotes":[{"name":"TANTIES","bounds":{"start":10,"end":17}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091214315,"data":"phnom penh is a hard thing to cold read imo","entities":{}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653091215117,"data":"Ph4t3 was that what was happening to pala? naughty you","entities":{"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653091229274,"data":"tahley that is why you read a script in advance TANTIES","entities":{"emotes":[{"name":"TANTIES","bounds":{"start":48,"end":55}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091258012,"data":"soadsoap PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":9,"end":18}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091262674,"data":"Gehirnchirurg we don\'t have time for that","entities":{"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091265204,"data":"Gehirnchirurg 1 and done","entities":{"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091266715,"data":"Gehirnchirurg TIMID","entities":{"emotes":[{"name":"TIMID","bounds":{"start":14,"end":19}}],"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"Gehirnchirurg","features":[],"timestamp":1653091282093,"data":"tahley PeepoRun:fast:smol money needs to be made","entities":{"emotes":[{"name":"PeepoRun","modifiers":["fast","smol"],"bounds":{"start":7,"end":25}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091299138,"data":"Gehirnchirurg it costs us $1000/min to have these people here","entities":{"nicks":[{"nick":"Gehirnchirurg","bounds":{"start":0,"end":13}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091300511,"data":"chop chop","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091312033,"data":"tfw you never get tied to Jessica Alba on a Thai island FeelsBadMan","entities":{"emotes":[{"name":"FeelsBadMan","bounds":{"start":56,"end":67}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091318839,"data":"Sadge","entities":{"emotes":[{"name":"Sadge","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091371034,"data":"I was supposed to reset yesterday but I fell asleep anyway Sadge","entities":{"emotes":[{"name":"Sadge","bounds":{"start":59,"end":64}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091403750,"data":"PepoDance","entities":{"emotes":[{"name":"PepoDance","bounds":{"start":0,"end":9}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091431067,"data":"soadsoap I skipped working out too and did absolutely nothing today too Sadge","entities":{"emotes":[{"name":"Sadge","bounds":{"start":72,"end":77}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091442970,"data":"soadsoap and rip schedule","entities":{"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"NevadaBama","features":[],"timestamp":1653091463944,"data":"Charles Barkley is the greatest","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091580300,"data":"Ph4t3 Sadge /","entities":{"emotes":[{"name":"Sadge","bounds":{"start":6,"end":11}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091592300,"data":"widdlez https://media.discordapp.net/attachments/284828648056422401/977341663624192020/unknown.png FeelsStrongMan unironically me","entities":{"links":[{"url":"https://media.discordapp.net/attachments/284828648056422401/977341663624192020/unknown.png","bounds":{"start":8,"end":98}}],"emotes":[{"name":"FeelsStrongMan","bounds":{"start":99,"end":113}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653091618444,"data":"soadsoap LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":9,"end":12}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091622359,"data":"soadsoap LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":9,"end":12}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"blankspaceblank","features":[],"timestamp":1653091660995,"data":"https://en.wikipedia.org/wiki/Saiga_antelope#/media/File:Saiga_antelope_at_the_Stepnoi_Sanctuary.jpg Avatar irl","entities":{"links":[{"url":"https://en.wikipedia.org/wiki/Saiga_antelope#/media/File:Saiga_antelope_at_the_Stepnoi_Sanctuary.jpg","bounds":{"start":0,"end":100}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653091674618,"data":"blankspaceblank nice snoot","entities":{"nicks":[{"nick":"blankspaceblank","bounds":{"start":0,"end":15}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653091693023,"data":"widdlez Ph4t3 me remembering my mom taught me how to use a spoon when she asks me what her email password is for the 900th time PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":128,"end":137}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}},{"nick":"Ph4t3","bounds":{"start":8,"end":13}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091833850,"data":"soadsoap I\'m glad to hear my mom is totally normal then PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":56,"end":65}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"arkzats","features":[],"timestamp":1653091993064,"data":"soadsoap Ph4t3 my mom somehow removed the outlook app from her ipad OsKrappa","entities":{"emotes":[{"name":"OsKrappa","bounds":{"start":68,"end":76}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}},{"nick":"Ph4t3","bounds":{"start":9,"end":14}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091994011,"data":"now there\'s Penang","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653091999639,"data":"now I know why she mixed it up PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":31,"end":40}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653092005901,"data":"how did they not edit that PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":27,"end":36}}]}}',
  'MSG {"nick":"tahley","features":[],"timestamp":1653092125377,"data":"|| BOOMIES https://youtu.be/WhE3iUH-TnI ||","entities":{"links":[{"url":"https://youtu.be/WhE3iUH-TnI","bounds":{"start":11,"end":39}}],"spoilers":[{"bounds":{"start":0,"end":42}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653092192927,"data":"arkzats OsKrappa","entities":{"emotes":[{"name":"OsKrappa","bounds":{"start":8,"end":16}}],"nicks":[{"nick":"arkzats","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653092355219,"data":"thanks random exposition guy","entities":{}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653092451069,"data":"widdlez crunches tooth https://media.discordapp.net/attachments/284828648056422401/977321688989565028/unknown.png modCheck:pause","entities":{"links":[{"url":"https://media.discordapp.net/attachments/284828648056422401/977321688989565028/unknown.png","bounds":{"start":23,"end":113}}],"emotes":[{"name":"modCheck","modifiers":["pause"],"bounds":{"start":114,"end":128}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653092476865,"data":"soadsoap modCheck:pause:wide","entities":{"emotes":[{"name":"modCheck","modifiers":["pause","wide"],"bounds":{"start":9,"end":28}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653092488351,"data":"soadsoap oof the pain SWEATY","entities":{"emotes":[{"name":"SWEATY","bounds":{"start":22,"end":28}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"BurningAmaranth","features":[],"timestamp":1653092647335,"data":"soadsoap PAIN","entities":{"emotes":[{"name":"PAIN","bounds":{"start":9,"end":13}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"BurningAmaranth","features":[],"timestamp":1653092656247,"data":"soadsoap did you reset? PauseChamp","entities":{"emotes":[{"name":"PauseChamp","bounds":{"start":24,"end":34}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653092907534,"data":"BurningAmaranth no DaFeels","entities":{"emotes":[{"name":"DaFeels","bounds":{"start":19,"end":26}}],"nicks":[{"nick":"BurningAmaranth","bounds":{"start":0,"end":15}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653092923433,"data":"Nept where\'d they get this picture of us https://media.discordapp.net/attachments/284828648056422401/977292837131542548/9372db85016caf35e77016db78e4ade813dec101.png","entities":{"links":[{"url":"https://media.discordapp.net/attachments/284828648056422401/977292837131542548/9372db85016caf35e77016db78e4ade813dec101.png","bounds":{"start":41,"end":164}}],"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653092932173,"data":"soadsoap PepoComfy","entities":{"emotes":[{"name":"PepoComfy","bounds":{"start":9,"end":18}}],"nicks":[{"nick":"soadsoap","bounds":{"start":0,"end":8}}]}}',
  'MSG {"nick":"skiitz","features":[],"timestamp":1653092963117,"data":"Nept peepoWave","entities":{"emotes":[{"name":"peepoWave","bounds":{"start":5,"end":14}}],"nicks":[{"nick":"Nept","bounds":{"start":0,"end":4}}]}}',
  'MSG {"nick":"Nept","features":[],"timestamp":1653093014972,"data":"skiitz peepoWave","entities":{"emotes":[{"name":"peepoWave","bounds":{"start":7,"end":16}}],"nicks":[{"nick":"skiitz","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093033371,"data":"does that pool actually exist somewhere?","entities":{}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093259195,"data":"it\'s real they just edited in the Sydney background WhoahDude https://www.villaamanzi.com/","entities":{"links":[{"url":"https://www.villaamanzi.com/","bounds":{"start":62,"end":90}}],"emotes":[{"name":"WhoahDude","bounds":{"start":52,"end":61}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093310875,"data":"just 5000$/night what a bargain GREED","entities":{"emotes":[{"name":"GREED","bounds":{"start":32,"end":37}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093348308,"data":"oh it\'s 2000$/night sorry","entities":{}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653093358532,"data":"that pool would terrify me LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":27,"end":30}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653093399317,"data":"tahley irl FeelsBadMan https://tenor.com/view/eli-made-this-gif-riddler-the-riddler-edward-nashton-edward-nygma-gif-25725781","entities":{"links":[{"url":"https://tenor.com/view/eli-made-this-gif-riddler-the-riddler-edward-nashton-edward-nygma-gif-25725781","bounds":{"start":23,"end":124}}],"emotes":[{"name":"FeelsBadMan","bounds":{"start":11,"end":22}}],"nicks":[{"nick":"tahley","bounds":{"start":0,"end":6}}]}}',
  'MSG {"nick":"soadsoap","features":[],"timestamp":1653093428542,"data":"This is not how this is supposed to go PepeHands","entities":{"emotes":[{"name":"PepeHands","bounds":{"start":39,"end":48}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093569996,"data":"widdlez it\'s not a glass bottom irl though","entities":{"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093588250,"data":"widdlez can\'t even skinny dip while someone is having breakfast in the apartment below me PepeHands","entities":{"emotes":[{"name":"PepeHands","bounds":{"start":90,"end":99}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"widdlez","features":[],"timestamp":1653093613063,"data":"Ph4t3 monkaMEGA but they exist https://www.travelandleisure.com/culture-design/architecture-design/embassy-gardens-sky-pool-london","entities":{"links":[{"url":"https://www.travelandleisure.com/culture-design/architecture-design/embassy-gardens-sky-pool-london","bounds":{"start":31,"end":130}}],"emotes":[{"name":"monkaMEGA","bounds":{"start":6,"end":15}}],"nicks":[{"nick":"Ph4t3","bounds":{"start":0,"end":5}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093681605,"data":"widdlez idk I think that\'s so dumb it\'s cool PepeLaugh","entities":{"emotes":[{"name":"PepeLaugh","bounds":{"start":45,"end":54}}],"nicks":[{"nick":"widdlez","bounds":{"start":0,"end":7}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093803665,"data":"puts on goggles underwater? LUL","entities":{"emotes":[{"name":"LUL","bounds":{"start":28,"end":31}}]}}',
  'MSG {"nick":"Ph4t3","features":[],"timestamp":1653093891957,"data":"the most comprehensive security system, a fence","entities":{}}',
  'MSG {"nick":"vitaran1","features":[],"timestamp":1653094082627,"data":"BOGGED","entities":{"emotes":[{"name":"BOGGED","bounds":{"start":0,"end":6}}]}}',
];

type LegacyMessage = {
  nick: string;
  timestamp: number;
  data: string;
  entities: Message.IEntities;
};

const historyMessages = history
  .map((v) => JSON.parse(v.substring(4)) as unknown as LegacyMessage)
  .map(
    ({ nick, timestamp, data, entities }) =>
      new Message({
        nick: nick,
        peerKey: new TextEncoder().encode(nick),
        // sentTime: BigInt(timestamp),
        serverTime: BigInt(timestamp),
        body: data,
        entities: new Message.Entities({
          ...entities,
          nicks: entities.nicks?.map((nick) => ({
            ...nick,
            peerKey: new TextEncoder().encode(nick.nick),
          })),
        }),
      })
  );

export const messages = historyMessages;

interface EmitterOptions {
  ivl?: number;
  batchSize?: number;
  limit?: number;
  preload?: number;
  messages?: Message[];
}

class Emitter extends PassThrough {
  tid: number;
  i = 0;

  constructor({
    ivl = 5000,
    batchSize = 1,
    limit = Infinity,
    preload = 0,
    messages = historyMessages,
  }: EmitterOptions) {
    super({ objectMode: true });

    let n = preload || 1;
    this.tid = window.setInterval(() => {
      if (this.i === limit) {
        clearInterval(this.tid);
        return;
      }

      const batch = [];
      while (n > 0) {
        const start = this.i % history.length;
        const end = Math.min(start + n, history.length);

        this.i += end - start;
        n -= end - start;

        batch.push(...messages.slice(start, end));
      }
      n = batchSize;

      this.push(batch);
    }, ivl);
  }

  destroy(e: Error) {
    clearInterval(this.tid);
    return super.destroy(e);
  }
}

export default Emitter;
