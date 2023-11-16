<script lang="ts">
	import { page } from '$app/stores';
	import { connected } from '$lib/updatesWS';

	const extHref = 'https://github.com/cooklang/cooklang-rs/blob/main/extensions.md';

	$: hostname = $page.url.hostname;
	$: local = hostname === 'localhost' || hostname === '127.0.0.1';
</script>

<h1>Über</h1>

<p>
	Zurzeit werden <a href="https://cooklang.org/">cooklang</a>
	Rezepte aus
	<span class="code">.cook</span> Dateien von {#if local}
		diesem Computer
	{:else}
		<span class="code">{hostname}</span>
	{/if} live bereitgestellt. Alles wird beim Laden der Seite umgesetzt, d. h. jede Änderung in den ursprünglichen Rezeptdateien wird in der Webanwendung wiedergegeben.
</p>

{#if $connected === 'connected'}
	<p>
		Der <span class="px-2 py-1 rounded border border-green-6 bg-green-3">grüne</span>
		Punkt oben rechts zeigt an, dass die Live-Aktualisierung funktioniert. Wenn du also eine
		<span class="code">.cook</span> Datei öffnest, Änderungen vornimmst und sie speicherst, wird das dargestellte Rezept automatisch aktualisiert.
	</p>
{:else}
	<p>
		Der <span class="px-2 py-1 rounded border border-red-6 bg-red-3">rote</span> Punkt oben rechts zeigt an, dass die Live-Updates im Moment nicht funktionieren. Du kannst versuchen, die Seite neu zu
		<button class="link" on:click={() => window.location.reload()}>laden</button>
		um die Verbindung zum Server wieder herzustellen. Bei Live-Updates werden Änderungen an den
		<span class="code">.cook</span> Dateien automatisch in dieser Webanwendung angezeigt.
	</p>
{/if}

<h2>
	Dies ist nicht das normale <a href="https://cooklang.org/">cooklang</a>
</h2>
<p>
	Diese Anwendung erweitert die reguläre Cooklang-Syntax. Die Erweiterungen bilden eine Obergruppe, so dass jedes ursprüngliche cooklang-Rezept immer noch genau dasselbe sein sollte. Du kannst eine Liste der Erweiterungen <a href={extHref} target="_blank">hier</a> einsehen.
</p>

<h2>Metadaten</h2>
<p>Diese Anwendung zeigt einige Metadaten an:</p>
<ul>
	<li>
		<span class="code">tags</span>: Kommagetrennte Liste von Schlagwörtern für das Rezept.
	</li>
	<li>
		<span class="code">emoji</span>: Fügen Sie ein Emoji hinzu, das zum Rezept passt.
	</li>
	<li>
		<span class="code">description</span>: Beschreibungstext.
	</li>
	<li id="author">
		<span class="font-bold">author</span>: Der Autor des Rezepts. Text mit dem Format:
		<ul>
			<li><span class="code">name</span></li>
			<li><span class="code">URL</span> (Es wird erkannt, dass es sich um eine URL handelt).</li>
			<li>
				<span class="code">name &lt;url&gt;</span> (Wenn beide angegeben werden, muss die URL innerhalb von &lt;&gt;).
			</li>
		</ul>
	</li>
	<li>
		<span class="font-bold">Source</span>: Die Originalquelle des Rezepts. Dasselbe wie
		<a href="#author">author</a>.
	</li>
	<li>
		<span class="font-bold">Time</span>: Die Vorbereitungs- und Kochzeit (oder kombinierte Zeit), die das Rezept benötigt.
		<ul>
			<li><span class="code">time</span>: Gesamtzeit</li>
			<li><span class="code">prep_time</span>: Vorbereitungszeit</li>
			<li><span class="code">cook_time</span>: Kochzeit</li>
		</ul>
	</li>
</ul>

<h2>Bilder</h2>
<p>
	Um Bilder hinzuzufügen, folgen Sie der <a href="https://cooklang.org/docs/spec/#adding-pictures"
		>cooklang Vorgabe</a
	>. Es gibt jedoch eine geringfügige Abweichung, da die Erweiterungen Abschnitte hinzufügen. Hier ist ein Beispiel für alle Optionen:
</p>
<pre class="bg-base-3 border border-base-6 rounded p-4">
Rezept.jpg        -- Hauptbild des Rezepts
Rezept.0.jpg      -- Bild für den ersten Abschnitt, erster Schritt
Rezept.0.0.jpg    -- wie oben
Rezept.2.0.jpg    -- Dritter Abschnitt, erster Schritt
</pre>
<p>
	Mit den <a href="{extHref}#text-steps" target="_blank">Text Schritten</a> wird auch der Index erhöht, so dass du Bilder zu diesen Absätzen hinzufügen kannst.
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
