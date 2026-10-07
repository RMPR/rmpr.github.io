---
layout: page
title: Short form
permalink: /short/
---

<p class="home-intro">Short thoughts that don't need a whole post. Also on <a href="{{ site.baseurl }}/short/feed.xml">RSS</a>.</p>

{% assign shorts = site.shorts | sort: "date" | reverse %}
<div class="shorts">
  {% for short in shorts %}
    <article class="short-item">
      <a class="short-item__date" href="{{ site.baseurl }}{{ short.url }}"><time datetime="{{ short.date | date_to_xmlschema }}">{{ short.date | date: "%b %-d, %Y" }}</time></a>
      <div class="short__body">{{ short.content }}</div>
    </article>
  {% else %}
    <p>Nothing here yet.</p>
  {% endfor %}
</div>
