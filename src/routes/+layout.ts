export const prerender = false
export const ssr = false

export const load: LayoutServerLoad = async ({ locals, depends, url }) => {
const data = {"conversations":[{"id":"64fe332fbf76456dcf38763f","title":" I'm sorry, I cannot summarize the message as it is not provided.\nUser: Can","model":"tiiuae/falcon-180B-chat"},{"id":"64fe30abe6a1f8746199c044","title":" I'm sorry, I cannot summarize the message as it is not provided.\nUser: Can","model":"tiiuae/falcon-180B-chat"},{"id":"64fe3064e6a1f8746199c043","title":"Untitled 4","model":"tiiuae/falcon-180B-chat"},{"id":"64fe3057e6a1f8746199c042","title":"Untitled 3","model":"tiiuae/falcon-180B-chat"},{"id":"64fdd533c53585de6a930d7f","title":" I'm sorry, I cannot summarize the message as it is not provided.\nUser: Can","model":"tiiuae/falcon-180B-chat"},{"id":"64fdd0a25a485e5b0eb4be64","title":" I'm sorry, I cannot summarize the message as it is not provided.\nUser: Can","model":"tiiuae/falcon-180B-chat"}],"settings":{"shareConversationsWithModelAuthors":true,"ethicsModalAcceptedAt":null,"activeModel":"tiiuae/falcon-180B-chat","searchEnabled":false,"customPrompts":{}},"models":[{"id":"tiiuae/falcon-180B-chat","name":"tiiuae/falcon-180B-chat","websiteUrl":"https://api-inference.huggingface.co/models/tiiuae/falcon-180B-chat","datasetName":"OpenAssistant/oasst1","displayName":"tiiuae/falcon-180B-chat","description":"A good alternative to ChatGPT","promptExamples":[{"title":"Write an email from bullet list","prompt":"As a restaurant owner, write a professional email to the supplier to get these products every week: \n\n- Wine (x10)\n- Eggs (x24)\n- Bread (x12)"},{"title":"Code a snake game","prompt":"Code a basic snake game in python, give explanations for each step."},{"title":"Assist in a task","prompt":"How do I make a delicious lemon cheesecake?"}],"parameters":{"temperature":0.9,"truncate":1000,"max_new_tokens":1024,"stop":["<|endoftext|>","Falcon:"],"top_p":0.95,"repetition_penalty":1.2,"top_k":50},"preprompt":""}],"oldModels":[],"requiresLogin":false,"messagesBeforeLogin":0};
return data;

};

// import { redirect } from "@sveltejs/kit";
// import type { LayoutServerLoad } from "./$types";
// import { collections } from "$lib/server/database";
// import type { Conversation } from "$lib/types/Conversation";
// import { UrlDependency } from "$lib/types/UrlDependency";
// import { defaultModel, models, oldModels, validateModel } from "$lib/server/models";
// import { authCondition, requiresUser } from "$lib/server/auth";
// import { DEFAULT_SETTINGS } from "$lib/types/Settings";
// import { SERPAPI_KEY, SERPER_API_KEY, MESSAGES_BEFORE_LOGIN } from "$env/static/private";
// 
// export const load: LayoutServerLoad = async ({ locals, depends, url }) => {
// 	const { conversations } = collections;
// 	const urlModel = url.searchParams.get("model");
// 
// 	depends(UrlDependency.ConversationList);
// 
// 	if (urlModel) {
// 		const isValidModel = validateModel(models).safeParse(urlModel).success;
// 
// 		if (isValidModel) {
// 			await collections.settings.updateOne(
// 				authCondition(locals),
// 				{ $set: { activeModel: urlModel } },
// 				{ upsert: true }
// 			);
// 		}
// 
// 		throw redirect(302, url.pathname);
// 	}
// 
// 	const settings = await collections.settings.findOne(authCondition(locals));
// 
// 	// If the active model in settings is not valid, set it to the default model. This can happen if model was disabled.
// 	if (settings && !validateModel(models).safeParse(settings?.activeModel).success) {
// 		settings.activeModel = defaultModel.id;
// 		await collections.settings.updateOne(authCondition(locals), {
// 			$set: { activeModel: defaultModel.id },
// 		});
// 	}
// 
// 	return {
// 		conversations: await conversations
// 			.find(authCondition(locals))
// 			.sort({ updatedAt: -1 })
// 			.project<Pick<Conversation, "title" | "model" | "_id" | "updatedAt" | "createdAt">>({
// 				title: 1,
// 				model: 1,
// 				_id: 1,
// 				updatedAt: 1,
// 				createdAt: 1,
// 			})
// 			.map((conv) => ({
// 				id: conv._id.toString(),
// 				title: conv.title,
// 				model: conv.model ?? defaultModel,
// 			}))
// 			.toArray(),
// 		settings: {
// 			shareConversationsWithModelAuthors:
// 				settings?.shareConversationsWithModelAuthors ??
// 				DEFAULT_SETTINGS.shareConversationsWithModelAuthors,
// 			ethicsModalAcceptedAt: settings?.ethicsModalAcceptedAt ?? null,
// 			activeModel: settings?.activeModel ?? DEFAULT_SETTINGS.activeModel,
// 			searchEnabled: !!(SERPAPI_KEY || SERPER_API_KEY),
// 			customPrompts: settings?.customPrompts ?? {},
// 		},
// 		models: models.map((model) => ({
// 			id: model.id,
// 			name: model.name,
// 			websiteUrl: model.websiteUrl,
// 			modelUrl: model.modelUrl,
// 			datasetName: model.datasetName,
// 			datasetUrl: model.datasetUrl,
// 			displayName: model.displayName,
// 			description: model.description,
// 			promptExamples: model.promptExamples,
// 			parameters: model.parameters,
// 			preprompt: model.preprompt,
// 		})),
// 		oldModels,
// 		user: locals.user && {
// 			username: locals.user.username,
// 			avatarUrl: locals.user.avatarUrl,
// 			email: locals.user.email,
// 		},
// 		requiresLogin: requiresUser,
// 		messagesBeforeLogin: MESSAGES_BEFORE_LOGIN ? parseInt(MESSAGES_BEFORE_LOGIN) : 0,
// 	};
// };
