---
layout: post
title: Approche guerilla au Google Summer of Code
---
Note préliminaire, si tu n'as aucune idée de ce qu'est le GSoC je te renvoie vers l'excellente
[page Wikipedia](https://fr.wikipedia.org/wiki/Google_Summer_of_Code) à ce sujet.
J'ai été contacté par un étudiant de mon ancienne école à propos de cette opportunité, somme toute assez
unique à laquelle j'ai participé il y'a quelques années déjà. J'aurais pu répondre directement, mais ce
serait hautement spécifique et je n'aurais pas vraiment eu le temps de rassembler toutes mes pensées à ce
sujet. Et étant donné qu'il n y'a pas beaucoup de documentation relative à ça en français, il m'a semblé qu'une
pièce de ce style serait assez utile. Il s'agit plus un mélimélo de choses que je pense plutôt que
quelque chose de vraiment structuré, un ensemble d'heuristiques pour permettre de
réduire drastiquement le nombre d'organisation/projets que tu auras à considérer pour ton choix (parce
que le choix t'appartient après tout).

## Avant de commencer
Il existe déjà de nombreux guides (en anglais pour la plupart sorry) qui parlent de comment aborder le
GSoC, ce qu'il faut faire, etc. Celui-ci est plus axé sur une approche guerilla où il faut faire les choses
bien et surtout faire les choses vite. Avant tout, si tu ne l'as pas déjà fait : installe une
distribution Linux... ne pose pas de questions fais le
juste. J'insiste, fais-le avant de continuer à lire cet article. Tu me remercieras plus tard. Je te conseillerais
Fedora juste comme ça, mais te sens pas obligé :p. Plus sérieusement, tu peux consulter
[distrochooser](https://distrochooser.de/fr/) pour t'aider et l'excellent cours d'openclassrooms
[reprendre le contrôle à l'aide de Linux](https://openclassrooms.com/fr/courses/43538-reprenez-le-controle-a-laide-de-linux).

## Choisir une organisation
Il faut noter le singulier: "une" organisation. L'erreur qui est facile à commettre c'est de se dire que
soumettre le maximum de proposals augmente la probabilité d'être sélectionné. Ce n'est pas nécessairement
vrai, et qui plus est, étant déjà en retard donc il faut être concentré. Quelques critères pour t'aider à te
décider plus vite:

### Technologie
Je mets cet aspect en premier parce que même étant enthousiaste à l'idée de travailler pour une
organisation, à la fin, la seule chose qui compte c'est être capable de pouvoir faire le travail
demandé. Le GSoC est certes une occasion d'apprentissage, mais on arrose pas une terre stérile, donc
mon conseil ici c'est de chercher des organisations qui manipulent des technologies avec lesquelles tu
as déjà une certaine expérience. Et pour être tout à fait précis, puisqu'il s'agit d'une situation de
crise, je dirais même qu'il faut choisir le langage avec lequel tu es le plus à l'aise et éliminer de
manière inconditionnelle
toutes les entreprises qui ne l'ont pas dans leur liste. Cet outil pourrait être utile dans cette
perspective: [Gsoc Organisation Scraper](https://github.com/rohithasrk/GSoC-Organisation-Scraper)

### Moyen de communication
Chaque organisation pour se coordonner a besoin d'un medium de communication, mon conseil ici est simple. Il faut
privilégier les compagnies qui ont les moyens de communication les moins accessibles, ou alors qui
requièrent un certain effort de configuration, IRC me vient directement à l'esprit. Non seulement il
y'aura forcément moins de populace, mais plus le medium est ancien plus l'entreprise est susceptible
d'être bien établie dans le monde de l'OSS ce qui m'amène au point suivant.

### Nombre de slots
Chaque organisation a un nombre maximal de participants qui lui est accordé par Google. Généralement cela
dépend de sa taille (et donc de son statut dans l'écosystème). Les plus anciens sont susceptibles d'avoir
le plus grand nombre de slots quoique pas toujours.

### Concurrence
Toujours regarder le nombre de participants déjà actifs dans l'organisation qu'on veut choisir. Moins il y'en
a, mieux c'est, évidemment. Mais attention, il faut regarder aux projets individuels parce que certaines
organisations ont une grande variété de projets, certains faciles (où beaucoup de gens vont s'agglutiner),
d'autres un peu moins évidents. Dans ce cas bien précis s'investir dans un projet certes plus difficile mais
avec peu ou pas d'affluence a un meilleur retour sur investissement.

## Choisir un projet
Une fois que le nombre d'organisations a été drastiquement réduit il est temps de passer au filtrage par
projet.

### Quel projet pour la proposal?
Il est possible de soumettre son propre projet sur lequel on va travailler durant le GSoC, encore faudrait-il
qu'il soit accepté. Mon conseil ici c'est, autant que possible, essaie de prendre un projet déjà existant. La
plupart des entreprises ont des wikis où ils ont une liste de projets qu'ils proposent. Déjà ça évite d'avoir
à débattre sur la pertinence du projet pour l'organisation, ensuite de débattre sur qui va te
mentorer (chaque projet a généralement un ou plusieurs mentors déjà arrêtés), et la formulation de ce
problème bien défini va t'aider dans la rédaction de ta proposal (sur laquelle tu dois travailler aussitôt
que tu as sélectionné un projet).

### The lower the better
Plus le projet est difficile et fait appel à des compétences rares ou des technologies pas sexy, moins il y'a
de personnes, et plus il y'a de chances d'être sélectionné. Par exemple l'IA c'est sexy, la programmation
système... un peu moins. Dans le même sens, tout ce qui a Javascript est très susceptible d'être déjà
surchargé, alors que C ou même Haskell... un peu moins. D'un autre côté il peut arriver que pour des projets nécessitant des skills non triviaux
il y'ait déjà quelqu'un travaillant dessus. Dans ce cas, il faut auditer. S'il est déjà avancé et fait
du bon travail, il faut laisser tomber et passer à autre chose. Sinon, dans l'éventualité où tu peux avoir
un meilleur rendement que lui, vas-y. Fun fact, sur smbcmp nous étions quelques 5 personnes, donc la
concurrence à un moment est peut-être inévitable.

## Prise finale
Voilà j'espère que ce sera utile, si d'autres choses me reviennent je ferais des addendum donc il faudra
consulter l'article de temps en temps. En ce qui concerne mon expérience, je l'ai plus ou moins relatée
[ici](https://rmpr.xyz/gsoc_2019/) et les liens vers ma proposal et le code sont
[ici](https://summerofcode.withgoogle.com/archive/2019/organizations/4908878360215552/). Il faut se rappeler
que ne pas être pris ce n'est pas la fin du monde, il faudra juste en tirer des leçons, et il existe d'autres
programmes de ce type auquel il est possible de postuler [Open Source Internships](https://github.com/deepanshu1422/List-Of-Open-Source-Internships-Programs)


Ressources supplémentaires:
- [Les 6 conseils d'Isaac Kamga pour être sélectionné au GSoC](https://cyprientankeu.com/2019/03/18/google-summer-of-code-2019-les-6-conseils-disaac-kamga-pour-etre-selectionne/)


