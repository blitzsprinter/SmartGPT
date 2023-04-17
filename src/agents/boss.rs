use std::error::Error;
use crate::{ProgramInfo, AgentLLMs, Agents, Message, agents::{process_response, LINE_WRAP, run_employee, Choice, try_parse}};
use colored::Colorize;

pub async fn run_boss(
    program: &mut ProgramInfo, task: &str, first_prompt: bool, feedback: bool,
) -> Result<String, Box<dyn Error>> {
    let ProgramInfo { context, plugins, personality, .. } = program;
    let Agents { boss, employee, .. } = &mut context.agents;

    if first_prompt {
        let commands = plugins.iter()
            .flat_map(|el| el.commands.iter())
            .map(|el| el.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        boss.prompt.push(Message::System(format!(
"You are The Boss, a large language model.

Personality: {}

You have been assigned one task by The Manager, a large language model. You will use your loose planning and adaptability to complete this task.
Your goal is to quickly and efficiently get the task done without refining it too much. You just want to get a sort of quicker, shallower answer.
Complete your task as quickly as possible.

You have access to one employee named The Employee, a large language model, who can run commands for you.
These commands are: {}

Your Employee is not meant to do detailed work, but simply to help you find information.

Only ask The Employee for one thing at a time.
Keep your Employee requests very simple.
Make sure to tell the Employee to save important information to files!

You cannot do anywork on your own. You will do all of your work through your Employee."
            , personality, commands
        )));
    }

    if feedback {
        boss.message_history.push(Message::User(format!(
"Hello, The Boss.

The Manager has provided you with the following feedback: {:?}

Continue to work with The Employee to complete your task based on this feedback.",
                task
            )));
    } else if first_prompt {
        boss.message_history.push(Message::User(format!(
"Hello, The Boss.

Your task is {:?}

Write a 2-sentence loose plan of how you will achieve this.",
                task
            )));
    } else {
        employee.prompt.clear();
        employee.message_history.clear();

        boss.message_history.push(Message::User(format!(
            "Hello, The Boss.
            
            Your task is {:?}

            Keep in mind that you have been given a new Employee. You may need to brief them on any details they need to complete their tasks.
            
            Write a 2-sentence loose plan of how you will achieve this.",
                task
        )));
    }

    let response = boss.model.get_response(&boss.get_messages()).await?;
    boss.message_history.push(Message::Assistant(response.clone()));

    let task_list = process_response(&response, LINE_WRAP);

    println!("{}", "BOSS".blue());
    println!("{}", "The boss has created a loose plan to achieve its goal.".white());
    println!();
    println!("{task_list}");
    println!();

    let mut new_prompt = true;

    loop {
        let ProgramInfo { context, .. } = program;
        let Agents { boss, .. } = &mut context.agents;

        boss.message_history.push(Message::User(
            "Create one simple request for The Employee. 
Do not give your employee specific commands, simply phrase your request with natural language.
Provide a very narrow and specific request for the Employee.
Remember: Your Employee is not meant to do detailed work, but simply to help you find information.
Make sure to tell the Employee to save important information to files!"
                .to_string()
        ));

        let response = boss.model.get_response(&boss.get_messages()).await?;
        let boss_request = process_response(&response, LINE_WRAP);

        println!("{}", "BOSS".blue());
        println!("{}", "The boss has assigned a task to its employee, The Employee.".white());
        println!();
        println!("{boss_request}");
        println!();

        let employee_response = run_employee(program, &boss_request, new_prompt).await?;
        new_prompt = false;

        let output = format!(
r#"The Employee has responded:
{}

You now have three choices.
A. I have finished the task The Manager provided me with. I shall report back with the information.
B. I have not finished the task. The Employee's response provided me with plenty of new information, so I will update my loose plan.
C. I have not finished the task. I shall proceed onto asking the Employee my next request.

Provide your response in this format:

reasoning: Reasoning
choice: A

Do not surround your response in code-blocks. Respond with pure YAML only.
"#,
        employee_response
);

        let ProgramInfo { context, .. } = program;
        let Agents { boss, .. } = &mut context.agents;

        boss.message_history.push(Message::User(output));
        
        let (response, choice): (_, Choice) = try_parse(boss, 3).await?;
        boss.message_history.push(Message::Assistant(response.clone()));
        let response = process_response(&response, LINE_WRAP);

        println!("{}", "BOSS".blue());
        println!("{}", "The boss has made a decision on whether to keep going.".white());
        println!();
        println!("{response}");
        println!();
    
        if choice.choice == "A" {
            boss.message_history.push(Message::User(
                "Provide The Manager with your work on completing the task, in at least one paragraph, ideally more.".to_string()
            ));

            let response = boss.model.get_response(&boss.get_messages()).await?;
            let boss_response = process_response(&response, LINE_WRAP);

            println!("{}", "BOSS".blue());
            println!("{}", "The boss has given The Manager a response..".white());
            println!();
            println!("{boss_response}");
            println!();

            boss.message_history.push(Message::Assistant(response.clone()));

            return Ok(response);
        }
    
        if choice.choice == "B" {
            boss.message_history.push(Message::User(
                "Write a new 2-sentence loose plan of how you will achieve your task.".to_string()
            ));

            let response = boss.model.get_response(&boss.get_messages()).await?;
            let boss_response = process_response(&response, LINE_WRAP);

            println!("{}", "BOSS".blue());
            println!("{}", "The boss has updated its plan.".white());
            println!();
            println!("{boss_response}");
            println!();

            boss.message_history.push(Message::Assistant(response.clone()));
        }
    }
}