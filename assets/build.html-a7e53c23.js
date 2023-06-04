import{_ as t}from"./plugin-vue_export-helper-c27b6911.js";import{r as i,o,c as r,a as e,b as n,d as a,w as u,e as s}from"./app-9cf1cd10.js";const c={},m=e("h2",{id:"use-a-local-sui-repo",tabindex:"-1"},[e("a",{class:"header-anchor",href:"#use-a-local-sui-repo","aria-hidden":"true"},"#"),n(" Use a local Sui repo")],-1),p=e("p",null,"If you build often, then repeating local file access is obviously faster than remote (and more reliable).",-1),v=e("p",null,'If you use the Rust SDK, replace your "git" dependencies with "path".',-1),b=e("p",null,'For Move dependencies replace "git" dependencies with "local".',-1),h=s('<h2 id="build-only-what-you-need" tabindex="-1"><a class="header-anchor" href="#build-only-what-you-need" aria-hidden="true">#</a> Build only what you need</h2><p>If you care only for the client, then do not build the whole thing. Do <code>cargo build -p sui</code> instead of <code>cargo build</code></p><h2 id="parallel-linker" tabindex="-1"><a class="header-anchor" href="#parallel-linker" aria-hidden="true">#</a> Parallel Linker</h2><p>Some build steps are not optimized for parallelism. Notably, you can see this with <code>top</code> on Linux (by pressing <kbd>1</kbd>) and you will see only one core busy while the linker is running.</p>',4),g=e("em",null,"may",-1),f={href:"https://github.com/rui314/mold",target:"_blank",rel:"noopener noreferrer"},x=s(`<p>After installation, you can enable for Rust by creating a <code>config.toml</code>. The following was verified to work for Sui built on Ubuntu:</p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code>$ cat ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = [&quot;-C&quot;, &quot;link-arg=-fuse-ld=mold&quot;]
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p>The performance gain varies widely, you have to try for yourself. Do not expect 10x faster... it accelerates only the link phase. Furthermore, the performance gap versus more recent GNU/LLVM linker release is closing.</p><h2 id="how-does-my-build-time-compare" tabindex="-1"><a class="header-anchor" href="#how-does-my-build-time-compare" aria-hidden="true">#</a> How does my build time compare?</h2><p>See some profiling below.</p><p>Measurements are for clean build of sui and sui-faucet only.</p><details class="hint-container details"><summary>Steps for measuring</summary><p>With suibase, do the following to get one measurement:</p><div class="language-bash line-numbers-mode" data-ext="sh"><pre class="language-bash"><code>$ localnet delete
$ localnet update
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div></div></div><p>If you do not have suibase, then do the following for the first measurements:</p><div class="language-bash line-numbers-mode" data-ext="sh"><pre class="language-bash"><code>$ <span class="token function">git</span> clone <span class="token parameter variable">-b</span> devnet https://github.com/MystenLabs/sui.git
$ <span class="token builtin class-name">cd</span> sui
$ <span class="token function">cargo</span> build <span class="token parameter variable">-p</span> sui <span class="token parameter variable">-p</span> sui-faucet
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p>... and get additional measurements with:</p><div class="language-bash line-numbers-mode" data-ext="sh"><pre class="language-bash"><code>$ <span class="token function">cargo</span> clean
$ <span class="token function">cargo</span> build <span class="token parameter variable">-p</span> sui <span class="token parameter variable">-p</span> sui-faucet
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div></div></div></details><p><strong>Modern Linux</strong><br></p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code>Low : Finished dev [unoptimized + debuginfo] target(s) in 2m 55s
High: Finished dev [unoptimized + debuginfo] target(s) in 2m 55s

Intel i7-13700K (16 Cores), 64 GB, NVMe PCIe 4
Ubuntu 22.10, Sui 0.31.2
Suibase 0.1.2
GCC 12.2 / Mold 1.11
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p><strong>M1 MAX macosx</strong><br></p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code>Low : Finished dev [unoptimized + debuginfo] target(s) in 4m 23
High: Finished dev [unoptimized + debuginfo] target(s) in 4m 24s

Apple M1 Max
macOS Ventura 13.3.1 (22E261), Sui 0.31.2
Suibase 0.1.2
Apple clang version 14.0.3
rustc 1.68.2
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p><strong>Old Server (~2013) Windows 10 WSL2</strong><br></p><div class="language-text line-numbers-mode" data-ext="text"><pre class="language-text"><code>Low : Finished dev [unoptimized + debuginfo] target(s) in 8m 06s
High: Finished dev [unoptimized + debuginfo] target(s) in 8m 20s

2xIntel Xeon E5-2697v2@2.7GHz(24 Cores), 64 GB, NVMe PCIe 3
WSL2 config: 32 VCore, 48 GB
Ubuntu 22.04, Sui 0.31.2
Suibase 0.1.2
GCC 11.3 / Mold 1.11
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div>`,13);function y(_,k){const l=i("RouterLink"),d=i("ExternalLinkIcon");return o(),r("div",null,[m,p,v,b,e("p",null,[n("For suibase users, see "),a(l,{to:"/how-to/scripts.html#faster-rust-and-move-build"},{default:u(()=>[n("here")]),_:1}),n(" to re-use its local repo already downloaded.")]),h,e("p",null,[n("One trick that "),g,n(" help is the parallel linker "),e("a",f,[n("Mold"),a(d)]),n(".")]),x])}const L=t(c,[["render",y],["__file","build.html.vue"]]);export{L as default};
