---
layout: post
title: A re-introduction to Javascript (ES6) - French
---

C'est juste une résumé (en français) de ce que j’ai compris de la lecture de l’article dont le lien est le suivant  :
[A re-introduction to Javascript](https://developer.mozilla.org/en-US/docs/Web/JavaScript/A_re-introduction_to_JavaScript)

# Structures de contrôle
Elles sont globalement semblables à celles des autres langages de la « famille C »

if, else, else if, while, do-while, for, switch.

Mais les nouveautés sont :

- le `for … of` pour itérer sur les structures itérables (array, string, etc.)
- le `for … in` pour itérer sur les propriétés d’un objet

```
for (let value of array){
// faire quelque chose
}

for (let property in object) {
 // faire quelque chose
}
```

# Les objets
on les déclare de deux façons :

```
let obj = new Object()
let obj = {}
```

Bien qu’il n’y ait pas tellement de différence, la deuxième syntaxe est préférable.

Avec la déclaration d’objet suivante :

```
let personne = {
nom: "Mauricius",
age: 20,
};
```
Pour accéder à ses diverses propriétés, on le fait également de deux façons

```
personne.nom  //Affiche 'Mauricius'
personne["nom"]  //Affiche 'Mauricius'
```

# Tableaux
On les déclare assez simplement :

```
let tableau = ['a', 'b', 'c']
tableau.push('d')   // Ajout d'un élément
et l’itération se fait avec la boucle for…of vue plus haut
```
```
for (const element of tableau) {
//Faire quelque chose avec element
}
```

# Fonctions
Globalement les fonctions sont aussi des objets, elles fonctionnent comme dans 
n’importe quel langage de la famille C, mais avec des ajouts : pas de restrictions
sur le nombre d’arguments; des spécificités : fonctions fléchées. Se référer au lien
au début pour plus de détails.
