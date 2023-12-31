<script lang="ts">
	import { createEventDispatcher } from "svelte";
    import { invoke } from '@tauri-apps/api/primitives'

	import Modal from "$lib/components/Modal.svelte";
	import CarbonClose from "~icons/carbon/close";
	import CarbonCheckmark from "~icons/carbon/checkmark-filled";
	import ModelCardMetadata from "./ModelCardMetadata.svelte";
	import type { Model } from "$lib/types/Model";
	import type { LayoutData } from "../../routes/$types";
	import { enhance } from "$app/forms";
	import { base } from "$app/paths";

	import CarbonEdit from "~icons/carbon/edit";
	import CarbonSave from "~icons/carbon/save";
	import CarbonRestart from "~icons/carbon/restart";

	export let settings: LayoutData["settings"];
	export let models: Array<Model>;

	let selectedModelId = settings.activeModel;

	const dispatch = createEventDispatcher<{ close: void }>();

	let expanded = false;

	function onToggle() {
		if (expanded) {
			settings.customPrompts[selectedModelId] = value;
		}
		expanded = !expanded;
	}

	let value = "";

	function onModelChange() {
		value =
			settings.customPrompts[selectedModelId] ??
			models.filter((el) => el.id === selectedModelId)[0].preprompt ??
			"";
	}

	$: selectedModelId, onModelChange();
</script>

<Modal width="max-w-lg" on:close>
	<form
		action="{base}/settings"
		method="post"
		on:submit={(e) => {
            const formData = new FormData(e.target)
            const data = {};
            for (let field of formData) {
              let [key, value] = field;
              try{
                value = JSON.parse(value)
              }catch(e){
              }
              if (value === ""){
                  value = null;
              }
              
              data[key] = value;
            }
            invoke("settings", {settings: data});

            console.log(data)
            // TODO this is not smooth... at all !
            window.location.reload();
			if (expanded) {
				onToggle();
			}
		}}
		use:enhance={() => {
			dispatch("close");
		}}
		class="flex w-full flex-col gap-5 p-6"
	>
		{#each Object.entries(settings).filter(([k]) => !(k == "activeModel" || k === "customPrompts")) as [key, val]}
			<input type="hidden" name={key} value={val} />
		{/each}
		<input type="hidden" name="customPrompts" value={JSON.stringify(settings.customPrompts)} />
		<div class="flex items-start justify-between text-xl font-semibold text-gray-800">
			<h2>Models</h2>
			<button type="button" class="group" on:click={() => dispatch("close")}>
				<CarbonClose class="text-gray-900 group-hover:text-gray-500" />
			</button>
		</div>

		<div class="space-y-4">
			{#each models as model}
				{@const active = model.id === selectedModelId}
				<div
					class="rounded-xl border border-gray-100 {active
						? 'bg-gradient-to-r from-primary-200/40 via-primary-500/10'
						: ''}"
				>
					<label class="group flex cursor-pointer p-3" on:change aria-label={model.displayName}>
						<input
							type="radio"
							class="sr-only"
							name="activeModel"
							value={model.id}
							bind:group={selectedModelId}
						/>
						<span>
							<span class="text-md block font-semibold leading-tight text-gray-800"
								>{model.displayName}</span
							>
							{#if model.description}
								<span class="text-xs text-[#9FA8B5]">{model.description}</span>
							{/if}
						</span>
						<CarbonCheckmark
							class="-mr-1 -mt-1 ml-auto shrink-0 text-xl {active
								? 'text-primary-400'
								: 'text-transparent group-hover:text-gray-200'}"
						/>
					</label>
					{#if active}
						<div class=" overflow-hidden rounded-xl px-3 pb-2">
							<div class="flex flex-row flex-nowrap gap-2 pb-1">
								<div class="text-xs font-semibold text-gray-500">System Prompt</div>
								{#if expanded}
									<button
										class="text-gray-500 hover:text-gray-900"
										on:click|preventDefault={onToggle}
									>
										<CarbonSave class="text-sm " />
									</button>
									<button
										class="text-gray-500 hover:text-gray-900"
										on:click|preventDefault={() => {
											value = model.preprompt ?? "";
										}}
									>
										<CarbonRestart class="text-sm " />
									</button>
								{:else}
									<button
										class=" text-gray-500 hover:text-gray-900"
										on:click|preventDefault={onToggle}
									>
										<CarbonEdit class="text-sm " />
									</button>
								{/if}
							</div>
							<textarea
								enterkeyhint="send"
								tabindex="0"
								rows="1"
								class="h-20 w-full resize-none scroll-p-3 overflow-x-hidden overflow-y-scroll rounded-md border border-gray-300 bg-transparent p-1  text-xs outline-none focus:ring-0 focus-visible:ring-0"
								bind:value
								hidden={!expanded}
							/>
						</div>
					{/if}
					<ModelCardMetadata {model} />
				</div>
			{/each}
		</div>
		<button
			type="submit"
			class="mt-2 rounded-full bg-black px-5 py-2 text-lg font-semibold text-gray-100 ring-gray-400 ring-offset-1 transition-colors hover:ring"
		>
			Apply
		</button>
	</form>
</Modal>
