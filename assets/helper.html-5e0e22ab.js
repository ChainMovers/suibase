import{_ as i,Y as l,Z as c,$ as s,a0 as n,a1 as a,a2 as r,a3 as t,E as e}from"./framework-1aca60a5.js";const u={},d=t(`<h2 id="setup" tabindex="-1"><a class="header-anchor" href="#setup" aria-hidden="true">#</a> Setup</h2><p>Call <code>~/suibase/pip-install</code> within any python virtual environment in which you want to use the API.</p><p>Example creating a new environment and installing the API:</p><div class="language-bash line-numbers-mode" data-ext="sh"><pre class="language-bash"><code>$ <span class="token builtin class-name">cd</span> ~/myproject
$ python3 <span class="token parameter variable">-m</span> venv <span class="token function">env</span>
$ <span class="token builtin class-name">.</span> env/bin/activate
$ ~/suibase/pip-install
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><h2 id="typical-usage" tabindex="-1"><a class="header-anchor" href="#typical-usage" aria-hidden="true">#</a> Typical Usage</h2><pre><code>1. import suibase;
2. Create an instance of suibase.Helper
3. Verify suibase is_installed()
4. select_workdir()
5. ... use the rest of the API ...
</code></pre><h2 id="api" tabindex="-1"><a class="header-anchor" href="#api" aria-hidden="true">#</a> API</h2><p>For now, there is no python documentation generated (work-in-progress).</p>`,8),k={href:"https://chainmovers.github.io/suibase-api-docs/suibase/struct.Helper.html",target:"_blank",rel:"noopener noreferrer"},v=t(`<p>There is only one class: <code>Helper</code></p><p>Some demo calls for each methods:</p><div class="language-python line-numbers-mode" data-ext="py"><pre class="language-python"><code>$ python3
Python <span class="token number">3.10</span><span class="token number">.6</span> <span class="token punctuation">(</span>main<span class="token punctuation">,</span> Mar <span class="token number">10</span> <span class="token number">2023</span><span class="token punctuation">,</span> <span class="token number">10</span><span class="token punctuation">:</span><span class="token number">55</span><span class="token punctuation">:</span><span class="token number">28</span><span class="token punctuation">)</span> <span class="token punctuation">[</span>GCC <span class="token number">11.3</span><span class="token number">.0</span><span class="token punctuation">]</span> on linux
Type <span class="token string">&quot;help&quot;</span><span class="token punctuation">,</span> <span class="token string">&quot;copyright&quot;</span><span class="token punctuation">,</span> <span class="token string">&quot;credits&quot;</span> <span class="token keyword">or</span> <span class="token string">&quot;license&quot;</span> <span class="token keyword">for</span> more information<span class="token punctuation">.</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> <span class="token keyword">import</span> suibase<span class="token punctuation">;</span>
<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token operator">=</span>suibase<span class="token punctuation">.</span>Helper<span class="token punctuation">(</span><span class="token punctuation">)</span><span class="token punctuation">;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>is_installed<span class="token punctuation">(</span><span class="token punctuation">)</span>
<span class="token boolean">True</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>select_workdir<span class="token punctuation">(</span><span class="token string">&quot;localnet&quot;</span><span class="token punctuation">)</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>workdir<span class="token punctuation">(</span><span class="token punctuation">)</span>
<span class="token string">&#39;localnet&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>keystore_pathname<span class="token punctuation">(</span><span class="token punctuation">)</span><span class="token punctuation">;</span>
<span class="token string">&#39;/home/user/suibase/workdirs/localnet/config/sui.keystore&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>client_address<span class="token punctuation">(</span><span class="token string">&quot;active&quot;</span><span class="token punctuation">)</span>
<span class="token string">&#39;0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>client_address<span class="token punctuation">(</span><span class="token string">&quot;sb-1-ed25519&quot;</span><span class="token punctuation">)</span><span class="token punctuation">;</span>
<span class="token string">&#39;0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>rpc_url<span class="token punctuation">(</span><span class="token punctuation">)</span>
<span class="token string">&#39;http://0.0.0.0:9000&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>ws_url<span class="token punctuation">(</span><span class="token punctuation">)</span>
<span class="token string">&#39;ws://0.0.0.0:9000&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>package_id<span class="token punctuation">(</span><span class="token string">&quot;demo&quot;</span><span class="token punctuation">)</span>
<span class="token string">&#39;0x794fc1d80f18a02eb0b7094d2f5a9f9f40bcf653996291f7a7086404689a19b5&#39;</span>

<span class="token operator">&gt;&gt;</span><span class="token operator">&gt;</span> helper<span class="token punctuation">.</span>published_new_objects<span class="token punctuation">(</span><span class="token string">&quot;demo::Counter::Counter&quot;</span><span class="token punctuation">)</span>
<span class="token punctuation">[</span><span class="token string">&#39;0xef876238524a33124a924aba5a141f2b317f1e61b12032e78fed5c6aba650093&#39;</span><span class="token punctuation">]</span>
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div><div class="line-number"></div></div></div><p>For the package_id and published_new_objects call to succeed, you have to first publish the package &#39;demo&#39; on localnet:</p><div class="language-bash line-numbers-mode" data-ext="sh"><pre class="language-bash"><code>$ localnet publish <span class="token parameter variable">--path</span> ~/suibase/rust/demo-app
</code></pre><div class="line-numbers" aria-hidden="true"><div class="line-number"></div></div></div>`,5);function b(m,h){const p=e("RouterLink"),o=e("ExternalLinkIcon");return l(),c("div",null,[s("p",null,[n("As needed, read first the "),a(p,{to:"/helpers.html"},{default:r(()=>[n("Helper Overview")]),_:1}),n(".")]),d,s("p",null,[n("The API very closely matches the "),s("a",k,[n("Rust API"),a(o)]),n(".")]),v])}const f=i(u,[["render",b],["__file","helper.html.vue"]]);export{f as default};
