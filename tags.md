---
layout: page
title: Tags
permalink: /tags/
---

{% assign sorted_tags = site.tags | sort %}

<div class="tag-cloud">
  {% for tag in sorted_tags %}
    <a href="#{{ tag[0] | slugify }}">#{{ tag[0] }} <span class="tag-count">{{ tag[1].size }}</span></a>
  {% endfor %}
</div>

{% for tag in sorted_tags %}
  <section class="tag-section" id="{{ tag[0] | slugify }}">
    <h2 class="tag-heading">#{{ tag[0] }}</h2>
    <div class="stream">
      {% for post in tag[1] %}
        <article class="stream__item">
          <time class="stream__date" datetime="{{ post.date | date_to_xmlschema }}">{{ post.date | date: "%b %-d, %Y" }}</time>
          <div class="stream__body">
            <div class="stream__title"><a href="{{ site.baseurl }}{{ post.url }}">{{ post.title }}</a></div>
          </div>
        </article>
      {% endfor %}
    </div>
  </section>
{% endfor %}
