<script>
  import { onMount } from "svelte";

  let username = "";
  let password = "";
  let invite_code = "";
  let oldUsername = "";
  let oldPassword = "";
  let newPassword = "";
  let message = "";
  let csrf_token = "";

  onMount(async () => {
    csrf_token = await (await fetch("/", {method: "POST"})).text();
  });

  async function register() {
    const res = await fetch("/api/register", {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({username, password, csrf_token, invite_code})
    });
    message = await res.text();
    username = "";
    password = "";
    invite_code = "";
  }

  async function changePassword() {
    const res = await fetch("/api/change_password", {
      method: "POST",
      headers: {"Content-Type": "application/json"},
      body: JSON.stringify({oldUsername, oldPassword, new_password: newPassword})
    });
    message = await res.text();
  }
</script>


<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    font-family: system-ui, sans-serif;
    background-color: var(--bg);
    color: var(--text);
    display: flex;
    justify-content: center;
    padding: 1rem;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #1e1e1e;
      --card-bg: #2a2a2a;
      --text: #eee;
      --input-bg: #333;
      --input-text: #fff;
      --button-bg: #4a90e2;
      --button-text: #fff;
    }
  }

  @media (prefers-color-scheme: light) {
    :root {
      --bg: #f5f5f5;
      --card-bg: #fff;
      --text: #111;
      --input-bg: #fff;
      --input-text: #111;
      --button-bg: #007bff;
      --button-text: #fff;
    }
  }

  main {
    width: 100%;
    max-width: 400px;
  }

  h1 {
    text-align: center;
    margin-bottom: 1.5rem;
  }

  section {
    background: var(--card-bg);
    padding: 1rem;
    margin-bottom: 1rem;
    border-radius: 0.5rem;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    display: flex;
    flex-direction: column;
  }

  h2 {
    margin-top: 0;
    font-size: 1.2rem;
    margin-bottom: 1rem;
  }

  input {
    width: 100%;
    padding: 0.5rem;
    margin-bottom: 0.8rem;
    border: 1px solid #ccc;
    border-radius: 0.25rem;
    background: var(--input-bg);
    color: var(--input-text);
    font-size: 1rem;
    word-break: break-word;
  }

  button {
    width: 100%;
    padding: 0.6rem;
    background: var(--button-bg);
    color: var(--button-text);
    border: none;
    border-radius: 0.25rem;
    font-size: 1rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  button:hover {
    filter: brightness(1.1);
  }

  p.message {
    text-align: center;
    margin-top: 1rem;
    word-break: break-word;
  }

  @media (max-width: 480px) {
    section {
      padding: 0.8rem;
    }

    h2 {
      font-size: 1rem;
    }

    button, input {
      font-size: 0.95rem;
    }
  }
</style>


<main>
  <h1>Matrix Portal</h1>

  <section>
    <h2>Register</h2>
    <input placeholder="Username" autocomplete="new-password" bind:value={username} />
    <input type="password" placeholder="Password" autocomplete="new-password" bind:value={password} />
    <input type="password" placeholder="Invite" autocomplete="new-password" bind:value={invite_code} />
    <button on:click={register}>Register</button>
  </section>

  <section>
    <h2>Change Password</h2>
    <input placeholder="Username" bind:value={oldUsername} />
    <input type="password" placeholder="Old Password" bind:value={oldPassword} />
    <input type="password" placeholder="New Password" bind:value={newPassword} />
    <button on:click={changePassword}>Change Password</button>
  </section>

  {#if message}
    <p class="message">{message}</p>
  {/if}
</main>