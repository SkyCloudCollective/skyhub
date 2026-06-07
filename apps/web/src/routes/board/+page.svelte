<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getRoadmap,
		listFeatureRequests,
		createFeatureRequest,
		voteFeature,
		sendFeedback,
		type RoadmapEntry,
		type FeatureRequest
	} from '$lib/api';

	let roadmap = $state<RoadmapEntry[]>([]);
	let requests = $state<FeatureRequest[]>([]);
	let offline = $state(false);

	let title = $state('');
	let body = $state('');
	let feedback = $state('');
	let feedbackSent = $state(false);

	async function load() {
		try {
			requests = await listFeatureRequests();
			offline = false;
		} catch {
			offline = true;
			return;
		}
		try {
			roadmap = await getRoadmap();
		} catch {
			roadmap = [];
		}
	}
	onMount(load);

	async function propose() {
		const t = title.trim();
		if (!t) return;
		await createFeatureRequest(t, body.trim() || undefined);
		title = '';
		body = '';
		requests = await listFeatureRequests();
	}

	async function toggleVote(r: FeatureRequest) {
		const res = await voteFeature(r.id, !r.voted);
		if (res) requests = requests.map((x) => (x.id === r.id ? { ...x, votes: res.votes, voted: res.voted } : x));
	}

	async function submitFeedback() {
		const m = feedback.trim();
		if (!m) return;
		await sendFeedback(m, '/board');
		feedback = '';
		feedbackSent = true;
		setTimeout(() => (feedbackSent = false), 4000);
	}
</script>

<header class="glass head">
	<h1>Board</h1>
	<p class="muted">
		Propose features, upvote what matters, and follow the public roadmap. Consultative democracy:
		members propose, the operator curates.
	</p>
</header>

{#if offline}
	<p class="glass pad warn">Library offline — start the API (just dev-api).</p>
{/if}

<div class="cols">
	<section class="main">
		<div class="glass pad">
			<h2>Propose a feature</h2>
			<form onsubmit={(e) => { e.preventDefault(); propose(); }}>
				<input placeholder="A short, clear title…" bind:value={title} aria-label="Feature title" />
				<textarea placeholder="Why it matters (optional)" bind:value={body} aria-label="Details" rows="2"></textarea>
				<button class="accent-fill" type="submit">Propose</button>
			</form>
		</div>

		<div class="glass pad">
			<h2>Requests</h2>
			<ul class="reqs">
				{#each requests as r (r.id)}
					<li>
						<button
							class="vote"
							class:voted={r.voted}
							onclick={() => toggleVote(r)}
							aria-pressed={r.voted}
							aria-label={`${r.voted ? 'Remove vote' : 'Upvote'} (${r.votes})`}
							title="Upvote"
						>
							<span class="caret" aria-hidden="true">▲</span>
							<span class="count">{r.votes}</span>
						</button>
						<div class="req-body">
							<div class="req-top">
								<span class="t">{r.title}</span>
								<span class="status {r.status}">{r.status}</span>
							</div>
							{#if r.body}<p class="muted desc">{r.body}</p>{/if}
							<p class="muted by">@{r.handle}</p>
						</div>
					</li>
				{:else}
					{#if !offline}<li class="muted empty">No requests yet — propose the first.</li>{/if}
				{/each}
			</ul>
		</div>
	</section>

	<aside class="side">
		<div class="glass pad">
			<h2>Roadmap</h2>
			<ul class="roadmap">
				{#each roadmap as e (e.id)}
					<li>
						<span class="rstatus {e.status}">{e.status}</span>
						<div>
							<span class="t">{e.title}</span>
							{#if e.eta}<span class="eta muted"> · {e.eta}</span>{/if}
							{#if e.body}<p class="muted desc">{e.body}</p>{/if}
						</div>
					</li>
				{:else}
					<li class="muted">The roadmap is being drafted.</li>
				{/each}
			</ul>
		</div>

		<div class="glass pad">
			<h2>Private feedback</h2>
			<p class="muted small">Goes straight to the operator — not public.</p>
			<form onsubmit={(e) => { e.preventDefault(); submitFeedback(); }}>
				<textarea placeholder="Anything on your mind…" bind:value={feedback} aria-label="Feedback" rows="3"></textarea>
				<button class="ghost" type="submit">Send</button>
			</form>
			{#if feedbackSent}<p class="sent">Thanks — received.</p>{/if}
		</div>
	</aside>
</div>

<style>
	.head {
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin-bottom: var(--space-1);
	}
	.pad {
		padding: var(--space-5);
	}
	.warn {
		color: var(--c-warn);
		margin-bottom: var(--space-4);
	}
	.cols {
		display: grid;
		grid-template-columns: 1fr 320px;
		gap: var(--space-4);
		align-items: start;
	}
	.main,
	.side {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	h2 {
		margin: 0 0 var(--space-3);
		font-size: var(--text-md);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	input,
	textarea {
		width: 100%;
		resize: vertical;
	}
	form button {
		align-self: flex-start;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.reqs li {
		display: flex;
		gap: var(--space-3);
		padding: var(--space-3) 0;
		border-top: 1px solid var(--line);
	}
	.vote {
		display: flex;
		flex-direction: column;
		align-items: center;
		min-width: 48px;
		padding: var(--space-1) var(--space-2);
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		color: var(--fg-dim);
		cursor: pointer;
	}
	.vote.voted {
		background: var(--accent);
		color: var(--accent-ink);
		border-color: transparent;
	}
	.vote .caret {
		font-size: var(--text-xs);
	}
	.vote .count {
		font-family: var(--font-mono);
		font-weight: 600;
	}
	.req-top {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.t {
		font-weight: 600;
	}
	.desc {
		margin: var(--space-1) 0 0;
		font-size: var(--text-sm);
	}
	.by {
		margin: var(--space-1) 0 0;
		font-size: var(--text-xs);
	}
	.status,
	.rstatus {
		font-size: var(--text-xs);
		padding: 1px var(--space-2);
		border-radius: var(--radius-pill);
		border: 1px solid var(--line);
		color: var(--muted);
		text-transform: lowercase;
	}
	.status.done,
	.rstatus.shipped {
		color: var(--c-good);
		border-color: color-mix(in srgb, var(--c-good) 40%, transparent);
	}
	.rstatus {
		align-self: start;
		white-space: nowrap;
	}
	.roadmap li {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-2) 0;
		border-top: 1px solid var(--line);
	}
	.roadmap li:first-child {
		border-top: 0;
	}
	.eta {
		font-size: var(--text-sm);
	}
	.small {
		font-size: var(--text-xs);
		margin-top: 0;
	}
	.empty {
		padding: var(--space-3) 0;
	}
	.sent {
		color: var(--c-good);
		font-size: var(--text-sm);
		margin: var(--space-2) 0 0;
	}
	@media (max-width: 820px) {
		.cols {
			grid-template-columns: 1fr;
		}
	}
</style>
