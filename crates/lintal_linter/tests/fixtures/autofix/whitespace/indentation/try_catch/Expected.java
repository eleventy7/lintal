import java.io.IOException;
import java.io.FileReader;
import java.io.BufferedReader;

public class Input {
    void simpleTryCatch() {
        try {
            System.out.println("try");
        } catch (Exception e) {
            System.out.println("catch");
        }
    }

    void tryCatchFinally() {
        try {
            System.out.println("try");
        } catch (Exception e) {
            System.out.println("catch");
        } finally {
           System.out.println("finally");
        }
    }

    void multiCatch() {
        try {
            throw new IOException();
        } catch (IOException | RuntimeException e) {
            System.out.println("multi catch");
        }
    }

    void nestedTry() {
        try {
            try {
                System.out.println("inner try");
            } catch (Exception inner) {
                System.out.println("inner catch");
            }
        } catch (Exception outer) {
            System.out.println("outer catch");
        }
    }

    void tryWithResources() throws Exception {
        try (BufferedReader reader = new BufferedReader(new FileReader("file.txt"))) {
            String line = reader.readLine();
            System.out.println(line);
        } catch (IOException e) {
            System.out.println("error");
        }
    }

    void complexTry() {
        try {
            if (true) {
                throw new IOException();
            }
        } catch (IOException e) {
            System.out.println("io error");
        } catch (RuntimeException e) {
            System.out.println("runtime error");
        } finally {
            if (true) {
             System.out.println("cleanup");
            }
        }
    }
}
