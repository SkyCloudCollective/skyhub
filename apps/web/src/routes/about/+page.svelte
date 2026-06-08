<script lang="ts">
	// A calm, static identity page for the SkyCollective. No data fetch.
</script>

<article class="about">
	<header class="glass hero">
		<span class="mark" aria-hidden="true"></span>
		<h1>The SkyCollective</h1>
		<p class="lead">
			A small community of musicians and builders sharing samples, instruments, and
			projects. SkyHub is our platform — community-owned, self-hostable, and free.
		</p>
	</header>

	<section class="grid">
		<div class="glass card">
			<h2>How it's steered</h2>
			<p>
				Consultative democracy: members propose and upvote on the <a href="/board">Board</a>,
				and a maintainer keeps the project coherent and curates the public roadmap. Your
				voice is visible; decisions are explained.
			</p>
		</div>

		<div class="glass card">
			<h2>How to take part</h2>
			<p>
				Contribute samples, start a <a href="/projects">project</a>, comment and react,
				follow other members, or send code. Propose an idea on the Board before building —
				it keeps the roadmap honest.
			</p>
		</div>

		<div class="glass card">
			<h2>Sovereign &amp; auditable</h2>
			<p>
				Licensed <strong>AGPL-3.0</strong>: a hosted instance must offer its source, so the
				platform stays a commons no one can quietly enclose. Static frontend, a single
				SQLite catalogue, reproducible builds.
			</p>
		</div>

		<div class="glass card">
			<h2>Calm by default</h2>
			<p>
				Built to be legible and low-pressure — warm contrast, reduced-motion honoured, no
				alarm iconography. Many of us are neurodivergent; accessibility is a first principle,
				not a checkbox.
			</p>
		</div>
	</section>

	<section class="glass instruments">
		<h2>The instruments</h2>
		<p>
			Two native instruments run in the browser <a href="/studio">studio</a> (WebAssembly) and
			as <strong>VST3 / CLAP</strong> plugins from one shared Rust DSP core — the web and the
			plugin run identical code.
		</p>
		<ul>
			<li><strong>Botanica</strong> — a granular sample-morph instrument. Sound design by <strong>Tev</strong>.</li>
			<li><strong>PhasePlan</strong> — a dual-wavetable subtractive synth with an open modulation matrix.</li>
		</ul>
	</section>

	<footer class="credits muted">
		<p>
			Original work, inspired by tools we love — no third-party code or assets. Sound design:
			<strong>Tev</strong>. Meters: <strong>polarity</strong> (MIT). Everything else under
			AGPL-3.0-or-later.
		</p>
	</footer>
</article>

<style>
	.about {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.hero {
		padding: var(--space-6);
		border-radius: var(--radius-md);
		text-align: center;
	}
	.mark {
		display: inline-block;
		width: 40px;
		height: 40px;
		border-radius: 9px;
		margin-bottom: var(--space-3);
		background: linear-gradient(
			135deg,
			var(--bauhaus-red) 0 33%,
			var(--bauhaus-yellow) 33% 66%,
			var(--bauhaus-blue) 66% 100%
		);
	}
	.hero h1 {
		margin: 0 0 var(--space-2);
	}
	.lead {
		max-width: 60ch;
		margin: 0 auto;
		color: var(--fg-dim);
		font-size: var(--text-md);
		line-height: var(--leading-loose, 1.7);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: var(--space-4);
	}
	.card,
	.instruments {
		padding: var(--space-5);
		border-radius: var(--radius-md);
	}
	h2 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-md);
	}
	p {
		margin: 0;
		color: var(--fg-dim);
		line-height: var(--leading-loose, 1.7);
	}
	.instruments ul {
		margin: var(--space-3) 0 0;
		padding-left: var(--space-5);
		color: var(--fg-dim);
		line-height: var(--leading-loose, 1.7);
	}
	.credits {
		padding: var(--space-4) var(--space-5);
	}
	.credits p {
		font-size: var(--text-sm);
	}
</style>
