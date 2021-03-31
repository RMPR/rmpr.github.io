---
layout: post
title: Characters array and String litterals in C
description: Characters array and String literals in C
---
Consider the following two snippets
```
char abc[] = "abc";
char xyz[] = "xyz";
char *p = abc;

printf("%c abc = %s\n", ++*p, abc);
p = xyz;
printf("%c xyz = %s\n", *p++, xyz);
```
```
char *abc = "abc";
char *xyz = "xyz";
char *p = abc;

printf("%c abc = %s\n", ++*p, abc);
p = xyz;
printf("%c xyz = %s\n", *p++, xyz);
```
What's the difference between them? One results in a segfault and the other doesn't.
A little explanation about why.
The statement `char abc[] = "abc"` creates a character array which is automatically null-terminated '\0', while `char *abc = "abc"` creates a string literal and any attempt to modify it results in an undefined
behaviour (which is in this case a segfault). Weirdly enough neither gcc's built-in static analyzer
`-fanalyzer` nor `scan-build` detects this.
Relevant [stackexchange answer](https://cs50.stackexchange.com/a/35486) (the accepted answer is not good).
.


