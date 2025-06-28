use async_openai::{Client, config::OpenAIConfig, types::CreateCompletionRequestArgs};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_local_llama32_demo() {
        let api_key = ""; //env!("OPENAI_API_KEY");
        let api_base = "http://localhost:11434/v1";
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base)
            .with_org_id("buciumede");

        let client = Client::with_config(config);

        {
            // let model = "gpt-3.5-turbo-instruct";
            let model = "llama3.2";
            let prompt = "Tell me the recipe of alfredo pasta";

            // Create a `CreateCompletionRequest`
            let request = CreateCompletionRequestArgs::default()
                .model(model)
                .prompt(prompt)
                .max_tokens(100_u32)
                .build()
                .unwrap();

            let response = client
                .completions()
                .create(request)
                .await
                .unwrap();
            println!("{}", response.choices.first().unwrap().text);
        }
    }
}
