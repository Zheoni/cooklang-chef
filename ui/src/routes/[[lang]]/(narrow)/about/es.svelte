<script lang="ts">
	import { page } from '$app/stores';
	import { connected } from '$lib/updatesWS';

	const extHref = 'https://github.com/cooklang/cooklang-rs/blob/main/extensions.md';

	$: hostname = $page.url.hostname;
	$: local = hostname === 'localhost' || hostname === '127.0.0.1';
</script>

<h1>Acerca de</h1>

<p>
	Esta aplicación muestra recetas <a href="https://cooklang.org">cooklang</a> desde archivos
	<span class="code">.cook</span>
	desde {#if local}
		el ordenador actual
	{:else}
		<span class="code">{hostname}</span>
	{/if} en vivo. Todo se renderiza cuando cargas la página, asi que cada cambio en la receta original
	será reflejado en la aplicaión web.
</p>

{#if $connected === 'connected'}
	<p>
		El punto <span class="px-2 py-1 rounded border border-green-6 bg-green-3">verde</span>
		arriba a la derecha indica que las actualizaciones en vivo están funcionando. Asi que si abres un
		fichero <span class="code">.cook</span>, haces cambios y lo guardas, la receta será actualizada
		automáticamente.
	</p>
{:else}
	<p>
		El punto <span class="px-2 py-1 rounded border border-red-6 bg-red-3">red</span> de arriba a la
		derecha indica que las actualizaciones en vivo no funcionan ahora mismo. Puedes intentarlo de
		nuevo
		<button class="link" on:click={() => window.location.reload()}>recargando</button> la página
		para intentar reconectarse con el servidor. Con las actualizaciones automáticas, los cambios que
		hagas en los archivos <span class="code">.cook</span> serán reflejados automáticamente en la aplicación
		web.
	</p>
{/if}

<h2>
	Esto no es <a href="https://cooklang.org/">cooklang</a> normal
</h2>
<p>
	Esta aplicación extiende la sintaxis normal de cooklang. Las extensiones forman un superconjunto,
	asi que las recetas originales de cooklang deberían mostrarse igual. Puedes ver una lista de las
	extensiones
	<a href={extHref} target="_blank">aqui</a>.
</p>

<h2>Metadatos</h2>
<p>Esta aplicación muestra los siguientes metadatos de manera especial:</p>
<ul>
	<li>
		<span class="code">tags</span>: (etiquetas) lista separada por comas de etiquetas de la receta.
	</li>
	<li>
		<span class="code">emoji</span>: Añade un emoji que encaje con la receta.
	</li>
	<li>
		<span class="code">description</span>: Texto descriptivo.
	</li>
	<li id="author">
		<span class="font-bold">Author</span>: Autor de la receta. Texto con el siguiente formato:
		<ul>
			<li><span class="code">name</span> (nombre)</li>
			<li><span class="code">URL</span> (Será detectado si es una URL).</li>
			<li>
				<span class="code">name &lt;url&gt;</span> (Cuando se dan los dos tienen que estar entre &lt;&gt;).
			</li>
		</ul>
	</li>
	<li>
		<span class="font-bold">Fuente</span>: La fuente original de la receta. Igual que el
		<a href="#author">autor</a>.
	</li>
	<li>
		<span class="font-bold">Tiempo</span>: El tiempo de preparación y cocinado (o ambos combinados)
		que tomará la receta.
		<ul>
			<li><span class="code">time</span>: tiempo total</li>
			<li><span class="code">prep_time</span>: tiempo de preparación</li>
			<li><span class="code">cook_time</span>: tiempo de cocinado</li>
		</ul>
	</li>
</ul>

<h2>Imágenes</h2>
<p>
	Para añadir imágenes sigue el <a href="https://cooklang.org/docs/spec/#adding-pictures"
		>convenio de cooklang</a
	>. Sin embarlo, hay un pequeño cambio debido a que las extensiones añaden secciones. Este es un
	ejemplo de las distintas opciones:
</p>
<pre class="bg-base-3 border border-base-6 rounded p-4">
Recipe.jpg        -- Imagen principal
Recipe.0.jpg      -- Imagen para la primera sección, primer paso
Recipe.0.0.jpg    -- Igual que arriba
Recipe.2.0.jpg    -- Tercetra sección, primer paso
</pre>
<p>
	Los <a href="{extHref}#text-steps" target="_blank">pasos de texto</a> también aumentan el índice del
	paso, asi que puedes añadir imágenes en ellos.
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
