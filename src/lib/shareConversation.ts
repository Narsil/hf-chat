import { base } from "$app/paths";
import { ERROR_MESSAGES, error } from "$lib/stores/errors";
import { share } from "./utils/share";

export async function shareConversation(id: string, title: string) {
	try {
		const res = await fetch(`${base}/conversation/${id}/share`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
		});

		if (!res.ok) {
			error.set("Error while sharing conversation, try again.");
			console.error("Error while sharing conversation: " + (await res.text()));
			return;
		}

		const { url } = await res.json();

		share(url, title);
	} catch (err) {
		error.set(ERROR_MESSAGES.default);
		console.error(err);
	}
}
