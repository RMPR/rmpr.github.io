---
layout: post
title: Integers in C or how to shoot yourself in the foot a thousand times
description: Integers in C or how to shoot yourself in the foot a thousand times
---

Quoting straight up from Effective C:

# Integer conversion rank

> No two signed integer types have the same rank, even if they have the same representation.
> The rank of a signed integer type is greater than the rank of any signed integer type with less precision.
> The rank of long long int is greater than the rank of long int , which is greater than the rank of int, which is greater than the rank of short int, which is greater than the rank of signed char
> The rank of any unsigned integer type equals the rank of the corresponding signed integer type, if any.
> The rank of char equals the rank of signed char and unsigned char .
> The rank of `_Bool` is less than the rank of all other standard integer types.
> The rank of any enumerated type equals the rank of the compatible integer type. Each enumerated type is compatible with char, a signed integer type, or an unsigned integer type.
> The rank of any extended signed integer type relative to another extended signed integer type with the same precision is implementation-defined but still subject to the other rules for determining the integer conversion rank.

# Integer Promotions

> A small type is an integer with a lower conversion rank than int or unsigned int . Integer promotion is the process of converting values of small types to an int or unsigned int . Integer promotions allow you to use an expression of a small type in any expression where an int or unsigned int may be used. For example, you could use the lower-ranked integer type—typically, char or short —on the right-hand side of an assignment or as an argument to a function.

Now consider this snippet:
```
unsigned ui = UINT_MAX;
signed char c = -1;
if (c == ui)
    printf("%d equals %u", c, ui);
else
    printf("Not equal");
```

What does it print? If you answered `-1 equals 4294967295` or some variant of that (depending on your platform) - Even though I'm still puzzled by the fact that int is 32 bits while I am on a 64 bit platform - you are correct.
But jeez what's going on?
Remember? First thing is integer promotion of `c` because signed char has a lower integer conversion rank.
Then because of the `==` operand `c` is converted to an unsigned and with the overflow rules it's now
UINT_MAX. Fortunately gcc and clang generates warning for that so if you compile using the -Wall flag
(thing that any sane person must do) you must be preserved of that kind of mistakes.
