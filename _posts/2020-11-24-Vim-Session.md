---
layout: post
title: Vim Sessions
---
Even though I prefer to use one buffer at a time to be more focused. I sometimes needed to have multiple buffers 
open in splits (Terminal, Netrw, ...). And when for whatever reason, I needed to close them or shutdown my computer. I didn't like loosing my context. Enter Vim sessions.

They are quite simple, if you want to save your layout as it is, just type `:mksession`  or for short `:mks` 
(yay! 6 strokes saved) and a file named Session.vim will be created. All you have to do the next time you open 
your project folder is `vim -S`. 

P.S. 
- If there's already a session file you will need to append ! at the end of the command to overwrite
- You can eventually specify the session filename, for more info `:help :mks`

In almost one year of Vim usage, I always wanted to do this, but somehow tutorials address mostly plugins installation and usage. No, I want the real, rough Vim, please.


[![Vim sessions](../images/sessions.gif)](https://www.youtube.com/watch?v=jPPozzOCyIw)

Edit:

I added the following remaps

```
" Save session 
nnoremap <leader>ss :mksession!<CR> 
" Reload session 
nnoremap <leader>sl :so Session.vim<CR>
```


