<script lang="ts">
	export let youtubeUrl: string | undefined | null;

	const youtubeRegex = /^(https?:\/\/)?(www\.)?youtube\.(com|es)\/watch\?v=([^&]*)$/;
	function getYoutubeId(url: string) {
		const r = url.match(youtubeRegex);
		if (r !== null && r[4]) return r[4];
		return null;
	}
	const youtubeVideoId = youtubeUrl && getYoutubeId(youtubeUrl);
</script>

{#if youtubeVideoId}
	<div class="embed print:display-none w-560px lg:w-700px xl:w-800px">
		<iframe
			src={`https://www.youtube-nocookie.com/embed/${youtubeVideoId}`}
			title="YouTube video player"
			frameborder="0"
			allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
			allowfullscreen
		/>
	</div>
{/if}

<style>
	.embed {
		position: relative;
		overflow: hidden;
		aspect-ratio: 16/9;
		max-width: 95%;
		margin-inline: auto;
	}

	.embed::after {
		display: block;
		content: '';
		padding-top: 56.25%;
	}

	.embed iframe {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}
</style>
