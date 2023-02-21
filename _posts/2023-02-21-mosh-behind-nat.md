---
layout: post
title: Connect to a server behind a NAT using mosh
---

If you're not familiar with [mosh](https://mosh.org/) it's essentially SSH but
with support for roaming and it works like a charm on slow connections. I
wanted to access my server which is behind a NAT, but with the current configs
it was not straightforward to do so. When you first launch mosh, sshd is launched
in the background then kills the connection, in order to circumvent that, edit the ssh
daemon config file: `/etc/ssh/sshd/config`and modify to add your port line:

```sh
Port #PORTNUMBER
```

If you want to change the port on a distribution with SELinux enabled, you have to tell SELinux
about this change.

```sh
semanage port -a -t ssh_port_t -p tcp #PORTNUMBER
```

After that, you need to forward the same port you'll be listening to, to your router.
Otherwise you won't get any data back. You can now access your server with:

```
mosh --ssh="ssh -p #PORTNUMBER" -p #PORTNUMBER username@serveradress
```
