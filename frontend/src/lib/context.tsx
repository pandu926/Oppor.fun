import {
  createContext,
  useContext,
  useEffect,
  useState,
  useRef,
  type ReactNode,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { demo, chain } from "./config";
import { post, request, setCsrf } from "./api";
import { demoWallet, loadDemo } from "./demo";
import { walletClient, switchChain, type Provider } from "./wallet";
import type { AdminProfile, SessionResult } from "./types";
type Session = { wallet: string; admin: boolean; provider: Provider | null };
type AppContextValue = {
  session: Session | null;
  connect: () => void;
  logout: () => Promise<void>;
  notify: (message: string) => void;
};
const AppContext = createContext<AppContextValue>(null!);
export const useApp = () => useContext(AppContext);
export function AppProvider({ children }: { children: ReactNode }) {
  const query = useQueryClient();
  const [session, setSession] = useState<Session | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [toast, setToast] = useState("");
  useEffect(() => {
    if (demo) loadDemo();
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(timer);
  }, [toast]);
  async function logout() {
    const previous = session;
    setSession(null);
    query.clear();
    const revocation =
      previous && !demo ? post("/auth/logout") : Promise.resolve();
    setCsrf(null);
    try {
      await revocation;
    } catch {
      /* Local logout still clears privileged UI. */
    }
  }
  useEffect(() => {
    const provider = session?.provider;
    if (!provider?.on) return;
    const changed = () => {
      void logout();
      setToast("Wallet changed. Connect and sign in again.");
    };
    provider.on("accountsChanged", changed);
    provider.on("chainChanged", changed);
    return () => {
      provider.removeListener?.("accountsChanged", changed);
      provider.removeListener?.("chainChanged", changed);
    };
  }, [session]);
  async function connectWallet() {
    setBusy(true);
    setError("");
    try {
      const provider = window.ethereum;
      if (!provider)
        throw new Error(
          "Install an Ethereum-compatible browser wallet to connect.",
        );
      const client = walletClient(provider);
      const [wallet] = await client.requestAddresses();
      if (!wallet) throw new Error("No wallet account selected.");
      if ((await client.getChainId()) !== chain.id) await switchChain(provider);
      const challenge = await post<{ message: string; expires_at: string }>(
        "/auth/challenge",
        { wallet, chain_id: String(chain.id) },
      );
      const signature = await client.signMessage({
        account: wallet,
        message: challenge.message,
      });
      const verified = await post<SessionResult>("/auth/verify", {
        message: challenge.message,
        signature,
      });
      setCsrf(verified.csrf_token);
      let admin = false;
      try {
        await request<AdminProfile>("/admin/me");
        admin = true;
      } catch {
        /* Admin is granted only by the server. */
      }
      setSession({ wallet: verified.wallet, admin, provider });
      setOpen(false);
      query.clear();
      setToast("Wallet connected. You are signed in.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connection failed.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <AppContext.Provider
      value={{
        session,
        connect: () => {
          setError("");
          setOpen(true);
        },
        logout,
        notify: setToast,
      }}
    >
      {children}
      {toast && (
        <div className="toast" role="status">
          {toast}
          <button
            aria-label="Dismiss notification"
            onClick={() => setToast("")}
          >
            ×
          </button>
        </div>
      )}
      {open && (
        <ConnectDialog close={() => setOpen(false)}>
          <h2 id="dialog-title">
            {demo ? "Explore Oppor" : "Connect your wallet"}
          </h2>
          <p>
            {demo
              ? "You are viewing sample campaigns. Try the participant and creator flows without using real funds."
              : "Sign in with your wallet. Signing a message does not authorize a token transfer."}
          </p>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          {demo ? (
            <button
              className="button primary wide"
              onClick={() => {
                setSession({
                  wallet: demoWallet,
                  admin: false,
                  provider: null,
                });
                setOpen(false);
                query.clear();
              }}
            >
              Continue in demo mode
            </button>
          ) : (
            <button
              className="button primary wide"
              disabled={busy}
              onClick={() => void connectWallet()}
            >
              {busy ? "Waiting for wallet…" : "Connect browser wallet"}
            </button>
          )}
          <p className="fineprint">
            {demo
              ? "No blockchain transactions will be sent."
              : "You will be asked to switch to the configured Arc network. Reconnect after a page reload to authorize changes."}
          </p>
        </ConnectDialog>
      )}
    </AppContext.Provider>
  );
}
function ConnectDialog({
  close,
  children,
}: {
  close: () => void;
  children: ReactNode;
}) {
  return <Dialog close={close}>{children}</Dialog>;
}
export function Dialog({
  close,
  children,
}: {
  close: () => void;
  children: ReactNode;
}) {
  const closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const box = document.getElementById("dialog");
    const focusable = () =>
      Array.from(
        box?.querySelectorAll<HTMLElement>(
          'button:not(:disabled),a[href],input,select,textarea,[tabindex="0"]',
        ) || [],
      );
    focusable()[0]?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeRef.current();
      if (event.key === "Tab") {
        const items = focusable(),
          first = items[0],
          last = items.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener("keydown", key);
    const old = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", key);
      document.body.style.overflow = old;
      previous?.focus();
    };
  }, []);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <section
        id="dialog"
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
      >
        <button
          className="icon-button close"
          aria-label="Close dialog"
          onClick={close}
        >
          ×
        </button>
        {children}
      </section>
    </div>
  );
}
