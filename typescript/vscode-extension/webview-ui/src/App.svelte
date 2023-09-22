<script lang="ts">
	import { provideVSCodeDesignSystem, vsCodeButton,allComponents } from "@vscode/webview-ui-toolkit";
	import { vscode } from "./utilities/vscode";

	// In order to use the Webview UI Toolkit web components they
	// must be registered with the browser (i.e. webview) using the
	// syntax below.
	//provideVSCodeDesignSystem().register(vsCodeButton());
	provideVSCodeDesignSystem().register(allComponents);

	// To register more toolkit components, simply import the component
	// registration function and call it from within the register
	// function, like so:
	//
	// provideVSCodeDesignSystem().register(
	//   vsCodeButton(),
	//   vsCodeCheckbox()
	// );
	//
	// Finally, if you would like to register all of the toolkit
	// components at once, there's a handy convenience function:
	//
	// provideVSCodeDesignSystem().register(allComponents);

	function handleHowdyClick() {
		vscode.postMessage({
			command: "hello",
			text: "Hey there partner! 🤠",
		});
	}
</script>

<main>
	<h1>Localnet</h1>
	<h1>Devnet</h1>
	<h1>Testnet</h1>
	<h1>Mainnet</h1>
	<vscode-button on:click={handleHowdyClick}>Howdy!</vscode-button>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: flex-start;
		height: 100%;
	}
</style>
