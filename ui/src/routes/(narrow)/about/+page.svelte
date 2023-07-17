<script lang="ts">
	import { page } from '$app/stores';
	import { connected } from '$lib/updatesWS';

	$: hostname = $page.url.hostname;
	$: local = hostname === 'localhost' || hostname === '127.0.0.1';
</script>

<h1>About</h1>

<p class="font-serif">
	This web application is part of the <a class="link" href="https://github.com/Zheoni/cooklang-rs"
		>cooklang-rs</a
	>
	project.
</p>

<p>
	It is currently serving and rendering <a href="https://cooklang.org/">cooklang</a>
	recipes from
	<span class="code">.cook</span> files from
	{#if local}
		the current computer
	{:else}
		<span class="code">{hostname}</span>
	{/if} live. Everything is rendered when you load the page, so every change in the original recipe files
	will be reflected in the web application.
</p>

{#if $connected === 'connected'}
	<p>
		The <span class="px-2 py-1 rounded border border-green-6 bg-green-3">green</span> dot in the top
		right indicates that live updates are working. So if you open a <span class="code">.cook</span> file,
		make changes and save it. The rendered recipe will be automatically updated.
	</p>
{/if}

<h2>
	This is not regular <a href="https://cooklang.org/">cooklang</a>
</h2>
<p>
	This application extends the regular cooklang syntax. The extensions form a superset, so every
	original cooklang recipe should still be the exact same. You can see a list of the extensions
	<a href="https://github.com/Zheoni/cooklang-rs/blob/main/docs/extensions.md" target="_blank"
		>here</a
	>.
</p>

<h2>Metadata</h2>
<p>This app renders some metadata values:</p>
<ul>
	<li>
		<span class="code">tags</span>: comma separated list of tags for the recipe.
	</li>
	<li>
		<span class="code">emoji</span>: Add an emoji that matches the recipe.
	</li>
	<li>
		<span class="code">description</span>: Description text.
	</li>
	<li id="author">
		<span class="font-bold">Author</span>: The author of the recipe. text with the format:
		<ul>
			<li><span class="code">name</span></li>
			<li><span class="code">URL</span> (It will be detected that it is an URL).</li>
			<li>
				<span class="code">name &lt;url&gt;</span> (When both given, the URL must be inside &lt;&gt;).
			</li>
		</ul>
	</li>
	<li>
		<span class="font-bold">Source</span>: The original source of the recipe. Same as
		<a href="#author">author</a>.
	</li>
	<li>
		<span class="font-bold">Time</span>: The prep and cook (or combined) time that the recipe will
		take.
		<ul>
			<li><span class="code">time</span>: total time</li>
			<li><span class="code">prep_time</span>: prep time</li>
			<li><span class="code">cook_time</span>: cook time</li>
		</ul>
	</li>
</ul>

<h2>Images</h2>
<p>
	To add images follow the <a href="https://cooklang.org/docs/spec/#adding-pictures"
		>cooklang convention</a
	>. However, this has a subtle change as the extensions add sections. Here is an example of all the
	options:
</p>
<pre class="bg-base-3 border border-base-6 rounded p-4">
Recipe.jpg        -- Main image of the recipe
Recipe.0.jpg      -- Image for first section, first step
Recipe.0.0.jpg    -- Same as above
Recipe.2.0.jpg    -- Third section, first step
</pre>
<p>
	The <a href="https://github.com/Zheoni/cooklang-rs/blob/main/docs/extensions.md#text-steps"
		>text steps</a
	> also increments the index, so you can add images to those paragraphs.
</p>

<style>
	h1,
	h2 {
		--at-apply: font-heading mb-4 mt-8;
	}

	h1 {
		--at-apply: text-6xl;
	}
	h2 {
		--at-apply: text-4xl;
	}

	p {
		--at-apply: my-4;
	}

	a {
		--at-apply: link;
	}

	ul {
		--at-apply: ml-6 list-disc;
	}

	li {
		--at-apply: my-3;
	}
</style>
